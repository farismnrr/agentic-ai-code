#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  printf 'release-build: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[[ "$#" -eq 1 ]] || fail 'usage: ops/release/build-artifacts.sh vX.Y.Z[-prerelease]'
[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?)$ ]] \
  || fail 'usage: ops/release/build-artifacts.sh vX.Y.Z[-prerelease]'
nuxt_version="${BASH_REMATCH[1]}"

target_list=(
  x86_64-unknown-linux-gnu
  x86_64-apple-darwin
  x86_64-pc-windows-gnu
)
web_manifests=(
  package.json
  packages/curl-tool/package.json
  packages/searxng-search-tool/package.json
  packages/terminal-tool/package.json
)
native_manifests=(
  packages/relay-agent/package.json
  packages/rust-tools/package.json
)

package_version() {
  node -p "require('./$1').version"
}

for manifest in "${web_manifests[@]}"; do
  actual="$(package_version "$manifest")"
  [[ "$actual" == "$nuxt_version" ]] \
    || fail "$manifest version is $actual, expected Nuxt version $nuxt_version"
done

metadata_json="$(cargo metadata \
  --manifest-path packages/rust-tools/Cargo.toml \
  --no-deps \
  --format-version 1 \
  --locked)"
rust_version="$(printf '%s' "$metadata_json" | node --input-type=module -e '
  import fs from "node:fs"
  const metadata = JSON.parse(fs.readFileSync(0, "utf8"))
  const packages = metadata.packages.filter((pkg) => pkg.name === "ai-tools")
  if (packages.length !== 1) process.exit(1)
  process.stdout.write(packages[0].version)
')" || fail 'unable to resolve the single ai-tools Cargo package version'
rust_package_name="$(printf '%s' "$metadata_json" | node --input-type=module -e '
  import fs from "node:fs"
  const metadata = JSON.parse(fs.readFileSync(0, "utf8"))
  const packages = metadata.packages.filter((pkg) => pkg.name === "ai-tools")
  if (packages.length !== 1) process.exit(1)
  process.stdout.write(packages[0].name)
')" || fail 'unable to resolve the ai-tools Cargo package'
[[ "$rust_package_name" == "ai-tools" ]] || fail 'the native release package must be named ai-tools'
[[ "$rust_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || fail "ai-tools version must be stable semver, got $rust_version"

for manifest in "${native_manifests[@]}"; do
  actual="$(package_version "$manifest")"
  [[ "$actual" == "$rust_version" ]] \
    || fail "$manifest version is $actual, expected ai-tools version $rust_version"
done

command -v rustup >/dev/null 2>&1 || fail 'rustup is required'
command -v file >/dev/null 2>&1 || fail 'file is required for target inspection'
command -v sha256sum >/dev/null 2>&1 || fail 'sha256sum is required'
command -v tar >/dev/null 2>&1 || fail 'tar is required for Linux/macOS packages'
command -v unzip >/dev/null 2>&1 || fail 'unzip is required for Windows package validation'
command -v zip >/dev/null 2>&1 || fail 'zip is required for Windows package creation'

for target in "${target_list[@]}"; do
  rustup target list --installed | grep -Fxq "$target" \
    || fail "Rust target $target is not installed; run: rustup target add $target"
done

host_target="$(rustc -vV | sed -n 's/^host: //p')"
[[ -n "$host_target" ]] || fail 'unable to determine the active Rust host target'

printf 'release-build: verifying web and native gates...\n'
pnpm exec nuxt prepare --dotenv .env.example
pnpm lint
pnpm typecheck
pnpm test
pnpm guardrail
pnpm audit:web
pnpm audit:rust

printf 'release-build: building Nuxt %s...\n' "$nuxt_version"
NUXT_SESSION_PASSWORD='build-only-session-password-not-for-runtime' \
NUXT_DATABASE_ENFORCE_LEAST_PRIVILEGE=false \
  pnpm exec nuxt build --dotenv .env.example

build_target() {
  local target="$1"
  printf 'release-build: building ai-tools %s for %s...\n' "$rust_version" "$target"
  if [[ "$target" == "$host_target" ]]; then
    cargo build \
      --manifest-path packages/rust-tools/Cargo.toml \
      --release \
      --locked \
      --target "$target" \
      --bin ai-tools
  else
    command -v cargo-zigbuild >/dev/null 2>&1 \
      || fail "cargo-zigbuild is required for non-host target $target"
    command -v zig >/dev/null 2>&1 \
      || fail "zig is required for non-host target $target"
    if [[ "$target" == "x86_64-pc-windows-gnu" ]]; then
      # aws-lc-sys ships verified x86_64 NASM objects for this target. Use
      # them explicitly so the release remains reproducible on Linux hosts
      # without installing a host NASM toolchain.
      AWS_LC_SYS_PREBUILT_NASM=1 cargo zigbuild \
        --manifest-path packages/rust-tools/Cargo.toml \
        --release \
        --locked \
        --target "$target" \
        --bin ai-tools
    else
      cargo zigbuild \
        --manifest-path packages/rust-tools/Cargo.toml \
        --release \
        --locked \
        --target "$target" \
        --bin ai-tools
    fi
  fi
}

binary_path() {
  local target="$1"
  if [[ "$target" == "x86_64-pc-windows-gnu" ]]; then
    printf '%s\n' "$ROOT/target/$target/release/ai-tools.exe"
  else
    printf '%s\n' "$ROOT/target/$target/release/ai-tools"
  fi
}

validate_binary() {
  local target="$1"
  local binary="$2"
  local format
  format="$(file -b "$binary")"
  case "$target" in
    x86_64-unknown-linux-gnu)
      [[ "$format" == *'ELF 64-bit'*'x86-64'* ]] \
        || fail "$binary is not an x86_64 Linux ELF binary: $format"
      ;;
    x86_64-apple-darwin)
      [[ "$format" == *'Mach-O 64-bit'*'x86_64'* ]] \
        || fail "$binary is not an x86_64 macOS Mach-O binary: $format"
      ;;
    x86_64-pc-windows-gnu)
      [[ "$format" == *'PE32+'*'x86-64'* ]] \
        || fail "$binary is not an x86_64 Windows PE binary: $format"
      ;;
    *)
      fail "unsupported release target $target"
      ;;
  esac
}

for target in "${target_list[@]}"; do
  build_target "$target"
  binary="$(binary_path "$target")"
  [[ -x "$binary" ]] || fail "expected executable not found: $binary"
  validate_binary "$target" "$binary"
  if [[ "$target" == "$host_target" ]]; then
    reported_version="$("$binary" --version)"
    [[ "$reported_version" == "ai-tools $rust_version" ]] \
      || fail "binary reports '$reported_version', expected 'ai-tools $rust_version'"
  fi
done

release_dir="$ROOT/dist/$tag"
staging_dir="$ROOT/target/release-package/$tag"
rm -rf -- "$release_dir" "$staging_dir"
mkdir -p "$release_dir" "$staging_dir"

release_files=()
for target in "${target_list[@]}"; do
  binary="$(binary_path "$target")"
  if [[ "$target" == "x86_64-pc-windows-gnu" ]]; then
    direct_name="ai-tools-$target.exe"
    package_name="ai-tools.exe"
    archive_name="ai-tools-v$rust_version-$target.zip"
    package_dir="$staging_dir/$target"
    mkdir -p "$package_dir"
    install -m 0755 "$binary" "$release_dir/$direct_name"
    install -m 0755 "$binary" "$package_dir/$package_name"
    (
      cd "$package_dir"
      zip -q -9 "$release_dir/$archive_name" "$package_name"
    )
    unzip -tq "$release_dir/$archive_name"
    unzip -Z1 "$release_dir/$archive_name" | grep -Fxq "$package_name" \
      || fail "$archive_name does not contain $package_name"
  else
    direct_name="ai-tools-$target"
    package_name="ai-tools"
    archive_name="ai-tools-v$rust_version-$target.tar.gz"
    package_dir="$staging_dir/$target"
    mkdir -p "$package_dir"
    install -m 0755 "$binary" "$release_dir/$direct_name"
    install -m 0755 "$binary" "$package_dir/$package_name"
    tar -C "$package_dir" -czf "$release_dir/$archive_name" "$package_name"
    tar -tzf "$release_dir/$archive_name" | grep -Fxq "$package_name" \
      || fail "$archive_name does not contain $package_name"
  fi
  release_files+=("$direct_name" "$archive_name")
done

head="$(git rev-parse HEAD)"
metadata_path="$release_dir/RELEASE-METADATA.json"
node --input-type=module -e '
  import fs from "node:fs"
  const [path, releaseTag, nuxtVersion, rustVersion, commit, ...targets] = process.argv.slice(1)
  const artifacts = targets.map((target) => {
    const windows = target === "x86_64-pc-windows-gnu"
    return {
      target,
      binary: "ai-tools-" + target + (windows ? ".exe" : ""),
      archive: "ai-tools-v" + rustVersion + "-" + target + "." + (windows ? "zip" : "tar.gz"),
    }
  })
  fs.writeFileSync(path, JSON.stringify({
    schemaVersion: 1,
    releaseTag,
    nuxtVersion,
    aiToolsVersion: rustVersion,
    commit,
    targets,
    artifacts,
  }, null, 2) + "\n")
' "$metadata_path" "$tag" "$nuxt_version" "$rust_version" "$head" "${target_list[@]}"
release_files+=("RELEASE-METADATA.json")

(
  cd "$release_dir"
  sha256sum "${release_files[@]}" > SHA256SUMS
  sha256sum --check SHA256SUMS
)

printf 'release-build: created release bundle for tag %s\n' "$tag"
printf '  Nuxt: %s\n' "$nuxt_version"
printf '  ai-tools: %s\n' "$rust_version"
printf '  commit: %s\n' "$head"
for file in "${release_files[@]}" SHA256SUMS; do
  printf '  %s\n' "$release_dir/$file"
done
