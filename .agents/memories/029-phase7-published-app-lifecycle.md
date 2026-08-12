# Plan 029 Phase 7 — published-app lifecycle

The frozen v1 catalog is `.agents/contracts/029-tool-catalog-v1.json`.
Its canonical sorted compact JSON SHA-256 is recorded below and checked by
`scripts/phase7-chatgpt-contract.sh`.

catalogSha256: `8b420d4efb658eb2e8fa50477b05611cfe84bc7c1eb8c69b88868487f38b1e63`

Tool names are public API identifiers after publication. Renames, removals,
new required properties, changed write/destructive semantics, coding-scope
changes, and risk-annotation changes require explicit review and republish.
Prefer additive optional properties and new tools.

After an approved change, run ChatGPT Developer Mode **Refresh** or recreate/
republish where required, re-run **Scan Tools**, inspect schemas/descriptions/
annotations, and review action controls. New actions are not assumed enabled.
Business workspaces may require recreate/republish; Enterprise/Edu use Refresh
followed by action-control review. Stale clients cannot grant new capabilities;
server authorization and the Plan 028 sandbox remain authoritative.
