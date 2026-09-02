#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  printf 'release-container: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[[ "$#" -eq 1 ]] || fail 'usage: ops/release/publish-container.sh vX.Y.Z[-prerelease]'
[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?)$ ]] \
  || fail 'usage: ops/release/publish-container.sh vX.Y.Z[-prerelease]'
nuxt_version="${BASH_REMATCH[1]}"
image="${GHCR_IMAGE:-ghcr.io/farismnrr/ai-code}"
platforms='linux/amd64,linux/arm64'
platform_list=(linux/amd64 linux/arm64)
web_manifests=(
  package.json
  packages/curl-tool/package.json
  packages/searxng-search-tool/package.json
  packages/terminal-tool/package.json
)

[[ "$image" =~ ^[a-z0-9./_-]+$ ]] || fail 'GHCR_IMAGE contains unsupported characters'
[[ "$(git branch --show-current)" == 'main' ]] || fail 'publish from main only'
[[ -z "$(git status --porcelain)" ]] || fail 'working tree must be clean'

head="$(git rev-parse HEAD)"
tag_head="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"
[[ "$tag_head" == "$head" ]] || fail "$tag must already exist locally and point at HEAD"
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1 \
  || fail "$tag must already be pushed to origin"

package_version() {
  node -p "require('./$1').version"
}

for manifest in "${web_manifests[@]}"; do
  actual="$(package_version "$manifest")"
  [[ "$actual" == "$nuxt_version" ]] \
    || fail "$manifest version is $actual, expected Nuxt version $nuxt_version"
done

command -v docker >/dev/null 2>&1 || fail 'Docker is required'
command -v jq >/dev/null 2>&1 || fail 'jq is required to read Buildx metadata'
docker info >/dev/null 2>&1 || fail 'Docker daemon is not available'
docker buildx version >/dev/null 2>&1 || fail 'Docker Buildx is required'

image_tags=("$tag" "$nuxt_version")
if [[ "$nuxt_version" == *-* ]]; then
  release_is_prerelease=true
else
  release_is_prerelease=false
  image_tags+=(latest)
fi

# Version tags identify a published container and must never be silently
# repointed. The stable latest alias is deliberately mutable.
for image_tag in "${image_tags[@]}"; do
  if [[ "$image_tag" != 'latest' ]] && docker buildx imagetools inspect "$image:$image_tag" >/dev/null 2>&1; then
    fail "GHCR tag $image:$image_tag already exists; refusing to overwrite it"
  fi
done

created="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
build_metadata="$ROOT/target/release-package/container-$tag/ghcr-build-metadata.json"
mkdir -p "$(dirname "$build_metadata")"
docker_tags=()
for image_tag in "${image_tags[@]}"; do
  docker_tags+=(--tag "$image:$image_tag")
done

printf 'release-container: pushing Nuxt %s to GHCR...\n' "$nuxt_version"
if ! docker buildx build \
  --platform "$platforms" \
  --build-arg "VERSION=$nuxt_version" \
  --build-arg "REVISION=$head" \
  --build-arg "CREATED=$created" \
  "${docker_tags[@]}" \
  --push \
  --metadata-file "$build_metadata" \
  .
then
  printf 'release-container: GHCR push failed\n' >&2
  exit 1
fi

digest="$(jq -r '."containerimage.digest" // empty' "$build_metadata")"
[[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] \
  || fail 'Buildx did not return a valid image digest'
immutable_ref="$image@$digest"

printf 'release-container: verifying pushed image %s...\n' "$immutable_ref"
inspect_output="$(docker buildx imagetools inspect "$immutable_ref")" \
  || fail 'GHCR immutable image inspection failed'
for platform in "${platform_list[@]}"; do
  grep -Fq "$platform" <<<"$inspect_output" \
    || fail "published image does not advertise $platform"
done
docker pull "$immutable_ref" >/dev/null
image_version="$(docker image inspect "$immutable_ref" --format '{{index .Config.Labels "org.opencontainers.image.version"}}')"
image_revision="$(docker image inspect "$immutable_ref" --format '{{index .Config.Labels "org.opencontainers.image.revision"}}')"
[[ "$image_version" == "$nuxt_version" ]] \
  || fail "pulled image label version is $image_version, expected $nuxt_version"
[[ "$image_revision" == "$head" ]] \
  || fail 'pulled image revision label does not match the reviewed main commit'

for image_tag in "${image_tags[@]}"; do
  tag_digest="$(docker buildx imagetools inspect "$image:$image_tag" --format '{{json .Manifest}}' | jq -r '.digest // empty')"
  [[ "$tag_digest" == "$digest" ]] \
    || fail "$image:$image_tag does not resolve to $digest"
done

printf 'release-container: published\n'
printf '  GHCR image: %s\n' "$image"
printf '  Nuxt version: %s\n' "$nuxt_version"
printf '  GHCR tags: %s\n' "${image_tags[*]}"
printf '  GHCR digest: %s\n' "$digest"
printf '  immutable image: %s\n' "$immutable_ref"
printf '  prerelease: %s\n' "$release_is_prerelease"
