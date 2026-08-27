#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  printf 'release-publish: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]] || fail 'usage: ops/release/publish.sh vX.Y.Z'
version="${BASH_REMATCH[1]}"
target="x86_64-unknown-linux-gnu"
image="${GHCR_IMAGE:-ghcr.io/farismnrr/ai-code}"
platform="linux/amd64"

[[ "$(git branch --show-current)" == "main" ]] || fail 'publish from main only'
[[ -z "$(git status --porcelain)" ]] || fail 'working tree must be clean'

head="$(git rev-parse HEAD)"
tag_head="$(git rev-list -n 1 "$tag" 2>/dev/null || true)"
[[ "$tag_head" == "$head" ]] || fail "$tag must already exist locally and point at HEAD"
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1 || fail "$tag must already be pushed to origin"

command -v gh >/dev/null 2>&1 || fail 'GitHub CLI (gh) is required'
gh auth status >/dev/null 2>&1 || fail 'GitHub CLI is not authenticated'
command -v docker >/dev/null 2>&1 || fail 'Docker is required'
docker info >/dev/null 2>&1 || fail 'Docker daemon is not available'
docker buildx version >/dev/null 2>&1 || fail 'Docker Buildx is required'

release_state="$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null || true)"
[[ "$release_state" != 'false' ]] || fail "published GitHub release $tag already exists"

"$ROOT/ops/release/build-artifacts.sh" "$tag"

release_dir="$ROOT/dist/$tag"
binary="$release_dir/ai-tools-$target"
archive="$release_dir/ai-tools-${tag}-${target}.tar.gz"
checksums="$release_dir/SHA256SUMS"
[[ -f "$binary" && -f "$archive" && -f "$checksums" ]] || fail 'release artifacts are missing after build'

if [[ "$release_state" == 'true' ]]; then
  printf 'release-publish: refreshing existing draft GitHub release...\n'
  gh release upload "$tag" "$binary" "$archive" "$checksums" --clobber
else
  printf 'release-publish: creating draft GitHub release...\n'
  gh release create "$tag" \
    "$binary" \
    "$archive" \
    "$checksums" \
    --verify-tag \
    --generate-notes \
    --title "AI Code $tag" \
    --draft
fi

revision="$head"
created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

printf 'release-publish: pushing linux/amd64 web image to GHCR...\n'
if ! docker buildx build \
  --platform "$platform" \
  --build-arg "VERSION=$version" \
  --build-arg "REVISION=$revision" \
  --build-arg "CREATED=$created" \
  --tag "$image:$tag" \
  --tag "$image:$version" \
  --tag "$image:latest" \
  --push \
  .
then
  printf 'release-publish: GHCR push failed; GitHub release %s remains draft\n' "$tag" >&2
  exit 1
fi

printf 'release-publish: publishing GitHub release...\n'
gh release edit "$tag" --draft=false

printf 'release-publish: published %s and %s:%s\n' "$tag" "$image" "$version"
