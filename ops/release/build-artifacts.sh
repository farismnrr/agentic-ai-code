#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  printf 'release-build: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]] || fail 'usage: ops/release/build-artifacts.sh vX.Y.Z'
version="${BASH_REMATCH[1]}"
target="x86_64-unknown-linux-gnu"

workspace_version="$(awk -F'"' '/^[[:space:]]*version = "/ { print $2; exit }' Cargo.toml)"
[[ "$workspace_version" == "$version" ]] || fail "Cargo workspace version is $workspace_version, expected $version"

root_version="$(node -p "require('./package.json').version")"
[[ "$root_version" == "$version" ]] || fail "package.json version is $root_version, expected $version"

for manifest in \
  packages/curl-tool/package.json \
  packages/relay-agent/package.json \
  packages/rust-tools/package.json \
  packages/searxng-search-tool/package.json \
  packages/terminal-tool/package.json
do
  package_version="$(node -p "require('./$manifest').version")"
  [[ "$package_version" == "$version" ]] || fail "$manifest version is $package_version, expected $version"
done

[[ "$(uname -s)" == "Linux" ]] || fail 'native release artifacts are supported from Linux only'
[[ "$(uname -m)" == "x86_64" ]] || fail "native relay release target is $target only"

printf 'release-build: verifying repository...\n'
pnpm lint
pnpm typecheck
pnpm test
pnpm guardrail

printf 'release-build: building Nuxt production output...\n'
pnpm exec nuxt build --dotenv .env.example

printf 'release-build: building ai-tools %s for %s...\n' "$version" "$target"
cargo build \
  --manifest-path packages/rust-tools/Cargo.toml \
  --release \
  --locked \
  --target "$target" \
  --bin ai-tools

binary="$ROOT/target/$target/release/ai-tools"
[[ -x "$binary" ]] || fail "expected executable not found: $binary"
reported_version="$($binary --version)"
[[ "$reported_version" == "ai-tools $version" ]] || fail "binary reports '$reported_version', expected 'ai-tools $version'"

release_dir="$ROOT/dist/$tag"
staging_dir="$ROOT/target/release-package/$tag"
binary_name="ai-tools-$target"
archive="ai-tools-${tag}-${target}.tar.gz"

rm -rf "$release_dir" "$staging_dir"
mkdir -p "$release_dir" "$staging_dir"
install -m 0755 "$binary" "$release_dir/$binary_name"
install -m 0755 "$binary" "$staging_dir/ai-tools"

tar -C "$staging_dir" -czf "$release_dir/$archive" ai-tools
(
  cd "$release_dir"
  sha256sum "$binary_name" "$archive" > SHA256SUMS
  sha256sum --check SHA256SUMS
)

printf 'release-build: created\n'
printf '  %s\n' "$release_dir/$binary_name"
printf '  %s\n' "$release_dir/$archive"
printf '  %s\n' "$release_dir/SHA256SUMS"
