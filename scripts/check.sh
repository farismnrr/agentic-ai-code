#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> Structural guardrail"
node scripts/guardrail.mjs sso-auth

echo "==> Production build check"
docker build   --target backend-builder   --file sso-auth/Dockerfile   --tag masih-awam/sso-auth:check   .

echo "==> All checks passed"
