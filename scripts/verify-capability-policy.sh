#!/usr/bin/env sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

cargo run --quiet --locked -p relay-core --example capability_policy_check
node --experimental-strip-types scripts/verify-capability-policy-behavior.ts
pnpm exec eslint shared/utils/capability-policy.ts app/components/chat/ChatToolApproval.vue
