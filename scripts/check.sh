#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT_DIR"

DOCKER_RUN_LIMITS=""

if [ -n "${CI_CPU_LIMIT:-}" ] && [ -n "${CI_MEMORY_LIMIT:-}" ]; then
  DOCKER_RUN_LIMITS="--cpus ${CI_CPU_LIMIT} --memory ${CI_MEMORY_LIMIT} --memory-swap ${CI_MEMORY_LIMIT}"
fi

build_target() {
  target="$1"
  tag="$2"

  if [ -n "${BUILDX_BUILDER:-}" ]; then
    docker buildx build \
      --builder "$BUILDX_BUILDER" \
      --platform linux/amd64 \
      --build-arg VITE_AGENTATION_ENABLED="${AGENTATION_ENABLED:-false}" \
      --target "$target" \
      --file sso-auth/Dockerfile \
      --tag "$tag" \
      --load \
      .
  else
    docker build \
      --build-arg VITE_AGENTATION_ENABLED="${AGENTATION_ENABLED:-false}" \
      --target "$target" \
      --file sso-auth/Dockerfile \
      --tag "$tag" \
      .
  fi
}

echo "==> Structural guardrail"
# shellcheck disable=SC2086
docker run --rm \
  $DOCKER_RUN_LIMITS \
  --volume "$ROOT_DIR:/workspace" \
  --workdir /workspace \
  node:22-bookworm-slim \
  node scripts/guardrail.mjs sso-auth

echo "==> Frontend typecheck, Rust format, and Clippy"
build_target backend-check masih-awam/sso-auth:check-quality

echo "==> Production build check"
build_target backend-builder masih-awam/sso-auth:check

echo "==> All checks passed"
