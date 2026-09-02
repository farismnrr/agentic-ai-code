#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  printf 'release-publish: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[[ "$#" -eq 1 ]] || fail 'usage: ops/release/publish.sh vX.Y.Z[-prerelease]'
[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?)$ ]] \
  || fail 'usage: ops/release/publish.sh vX.Y.Z[-prerelease]'
cli_version="${BASH_REMATCH[1]}"

[[ "$(git branch --show-current)" == "main" ]] || fail 'publish from main only'
[[ -z "$(git status --porcelain)" ]] || fail 'working tree must be clean'

head="$(git rev-parse HEAD)"
tag_head="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"
[[ "$tag_head" == "$head" ]] || fail "$tag must already exist locally and point at HEAD"
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1 \
  || fail "$tag must already be pushed to origin"

rust_version="$(cargo metadata \
  --manifest-path packages/rust-tools/Cargo.toml \
  --no-deps \
  --format-version 1 \
  --locked \
  | node --input-type=module -e '
    import fs from "node:fs"
    const metadata = JSON.parse(fs.readFileSync(0, "utf8"))
    const packages = metadata.packages.filter((pkg) => pkg.name === "ai-tools")
    if (packages.length !== 1) process.exit(1)
    process.stdout.write(packages[0].version)
  ')" || fail 'unable to resolve the single ai-tools Cargo package version'
[[ "$rust_version" == "$cli_version" ]] \
  || fail "ai-tools version is $rust_version, expected CLI release version $cli_version"

command -v gh >/dev/null 2>&1 || fail 'GitHub CLI (gh) is required'
gh auth status >/dev/null 2>&1 || fail 'GitHub CLI is not authenticated'
command -v jq >/dev/null 2>&1 || fail 'jq is required for release metadata validation'

release_state="$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null || true)"
[[ "$release_state" != 'false' ]] || fail "published GitHub release $tag already exists"

"$ROOT/ops/release/build-artifacts.sh" "$tag"

release_dir="$ROOT/dist/$tag"
checksums="$release_dir/SHA256SUMS"
metadata="$release_dir/RELEASE-METADATA.json"
[[ -f "$checksums" && -f "$metadata" ]] \
  || fail 'release checksums or metadata are missing after build'
(
  cd "$release_dir"
  sha256sum --check SHA256SUMS
)
[[ "$(jq -r '.releaseTag // empty' "$metadata")" == "$tag" ]] \
  || fail 'release metadata tag does not match the requested CLI release'
[[ "$(jq -r '.aiToolsVersion // empty' "$metadata")" == "$cli_version" ]] \
  || fail 'release metadata version does not match the requested CLI release'

mapfile -t asset_names < <(find "$release_dir" -maxdepth 1 -type f -printf '%f\n' | sort)
(( "${#asset_names[@]}" == 8 )) || fail 'release bundle must contain exactly eight CLI release assets'
release_assets=()
for asset_name in "${asset_names[@]}"; do
  release_assets+=("$release_dir/$asset_name")
done

if [[ "$release_state" == 'true' ]]; then
  printf 'release-publish: refreshing existing draft GitHub release...\n'
  gh release upload "$tag" "${release_assets[@]}" --clobber
else
  printf 'release-publish: creating draft GitHub release...\n'
  release_flags=(--draft --verify-tag --generate-notes --title "AI Code $tag")
  [[ "$cli_version" == *-* ]] && release_flags+=(--prerelease)
  gh release create "$tag" "${release_assets[@]}" "${release_flags[@]}"
fi

printf 'release-publish: publishing CLI GitHub release...\n'
release_edit_flags=(--draft=false)
[[ "$cli_version" == *-* ]] && release_edit_flags+=(--prerelease)
gh release edit "$tag" "${release_edit_flags[@]}"

printf 'release-publish: published\n'
printf '  CLI GitHub release: %s\n' "$tag"
printf '  ai-tools version: %s\n' "$cli_version"
printf '  assets: %s\n' "${#asset_names[@]}"
gh release view "$tag" --json url --jq '.url'
