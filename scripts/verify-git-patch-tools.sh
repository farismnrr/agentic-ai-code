#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel); tmp="$root/.tmp/039b-acceptance"; rm -rf "$tmp"; mkdir -p "$tmp/repo"; trap 'rm -rf "$tmp"' EXIT
cd "$tmp/repo"; git init -q; git config user.email fixture@example.test; git config user.name fixture; printf 'one\ntwo\nthree\n' > sample.txt; git add sample.txt; git commit -qm init
marker="$tmp/executed"; git config diff.evil.command "sh -c 'touch $marker'"; git config diff.external "sh -c 'touch $marker'"; git config core.fsmonitor "sh -c 'touch $marker'"; git config core.pager "sh -c 'touch $marker'"; printf '*.txt diff=evil\n' > .gitattributes; printf 'changed\n' >> sample.txt
# Static contract: fixed git process config neutralizes executable helpers and public catalog exposes only read Git plus apply_patch.
rg -q 'core.fsmonitor=false' "$root/packages/rust-tools/application/src/git/process.rs"
rg -q 'diff.external=' "$root/packages/rust-tools/application/src/git/process.rs"
rg -q 'GIT_CONFIG_GLOBAL.*dev/null' "$root/packages/rust-tools/application/src/git/process.rs"
! rg -q 'name: "git_(commit|push|merge|rebase|reset|checkout|switch|clean)"' "$root/packages/rust-tools/interfaces/src/mcp/catalog.rs"
for name in git_status git_diff git_log git_show git_blame apply_patch; do rg -q "name: \"$name\"" "$root/packages/rust-tools/interfaces/src/mcp/catalog.rs"; done
# Direct Git flags used by adapter must not execute hostile diff helpers.
env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/nonexistent LC_ALL=C GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0 GIT_PAGER=cat PAGER=cat git -c core.pager=cat -c core.fsmonitor=false -c core.hooksPath=/dev/null -c diff.external= -c diff.trustExitCode=false -c color.ui=false diff --no-ext-diff --no-textconv -- sample.txt >/dev/null
[[ ! -e "$marker" ]]
# Patch grammar rejects adds/deletes and traversal by construction.
rg -q 'patch rename/add/delete is unsupported' "$root/packages/rust-tools/application/src/workspace/patch.rs"
rg -q 'patch path is invalid' "$root/packages/rust-tools/application/src/workspace/patch.rs"
echo 'git/patch safety: PASS'
