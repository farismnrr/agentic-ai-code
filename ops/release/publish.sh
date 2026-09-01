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
nuxt_version="${BASH_REMATCH[1]}"
image="${GHCR_IMAGE:-ghcr.io/farismnrr/ai-code}"
platform='linux/amd64'

[[ "$image" =~ ^[a-z0-9./_-]+$ ]] || fail 'GHCR_IMAGE contains unsupported characters'
[[ "$(git branch --show-current)" == "main" ]] || fail 'publish from main only'
[[ -z "$(git status --porcelain)" ]] || fail 'working tree must be clean'

head="$(git rev-parse HEAD)"
tag_head="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"
[[ "$tag_head" == "$head" ]] || fail "$tag must already exist locally and point at HEAD"
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1 \
  || fail "$tag must already be pushed to origin"

root_version="$(node -p "require('./package.json').version")"
[[ "$root_version" == "$nuxt_version" ]] \
  || fail "package.json version is $root_version, expected Nuxt version $nuxt_version"

command -v gh >/dev/null 2>&1 || fail 'GitHub CLI (gh) is required'
gh auth status >/dev/null 2>&1 || fail 'GitHub CLI is not authenticated'
command -v docker >/dev/null 2>&1 || fail 'Docker is required'
docker info >/dev/null 2>&1 || fail 'Docker daemon is not available'
docker buildx version >/dev/null 2>&1 || fail 'Docker Buildx is required'

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

mapfile -t asset_names < <(find "$release_dir" -maxdepth 1 -type f -printf '%f\n' | sort)
(( "${#asset_names[@]}" >= 4 )) || fail 'release bundle contains too few assets'
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
  [[ "$nuxt_version" == *-* ]] && release_flags+=(--prerelease)
  gh release create "$tag" "${release_assets[@]}" "${release_flags[@]}"
fi

created="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
build_metadata="$ROOT/target/release-package/$tag/ghcr-build-metadata.json"
mkdir -p "$(dirname "$build_metadata")"

printf 'release-publish: pushing %s web image to GHCR...\n' "$platform"
if ! docker buildx build \
  --platform "$platform" \
  --build-arg "VERSION=$nuxt_version" \
  --build-arg "REVISION=$head" \
  --build-arg "CREATED=$created" \
  --tag "$image:$tag" \
  --tag "$image:$nuxt_version" \
  --push \
  --metadata-file "$build_metadata" \
  .
then
  printf 'release-publish: GHCR push failed; GitHub release %s remains draft\n' "$tag" >&2
  exit 1
fi

command -v jq >/dev/null 2>&1 || fail 'jq is required to read Buildx metadata'
digest="$(jq -r '."containerimage.digest" // empty' "$build_metadata")"
[[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] \
  || fail 'Buildx did not return a valid image digest'
immutable_ref="$image@$digest"

printf 'release-publish: verifying pushed image %s...\n' "$immutable_ref"
inspect_output="$(docker buildx imagetools inspect "$immutable_ref")" \
  || fail 'GHCR immutable image inspection failed'
grep -Fq 'linux/amd64' <<<"$inspect_output" \
  || fail 'published image does not advertise linux/amd64'
docker pull "$immutable_ref" >/dev/null
image_version="$(docker image inspect "$immutable_ref" --format '{{index .Config.Labels "org.opencontainers.image.version"}}')"
image_revision="$(docker image inspect "$immutable_ref" --format '{{index .Config.Labels "org.opencontainers.image.revision"}}')"
[[ "$image_version" == "$nuxt_version" ]] \
  || fail "pulled image label version is $image_version, expected $nuxt_version"
[[ "$image_revision" == "$head" ]] \
  || fail 'pulled image revision label does not match the reviewed main commit'

for image_tag in "$tag" "$nuxt_version"; do
  tag_digest="$(docker buildx imagetools inspect "$image:$image_tag" --format '{{json .Manifest}}' | jq -r '.digest // empty')"
  [[ "$tag_digest" == "$digest" ]] \
    || fail "$image:$image_tag does not resolve to $digest"
done

printf 'release-publish: publishing GitHub release...\n'
release_edit_flags=(--draft=false)
[[ "$nuxt_version" == *-* ]] && release_edit_flags+=(--prerelease)
gh release edit "$tag" "${release_edit_flags[@]}"

printf 'release-publish: published\n'
printf '  GitHub release: %s\n' "$tag"
printf '  GHCR image: %s\n' "$image"
printf '  GHCR tags: %s, %s\n' "$tag" "$nuxt_version"
printf '  GHCR digest: %s\n' "$digest"
printf '  immutable image: %s\n' "$immutable_ref"
