# Plan 029 Phase 7 — published-app lifecycle

The frozen v1 catalog is `.agents/contracts/029-tool-catalog-v1.json`.
Its canonical sorted compact JSON SHA-256 is recorded below and checked by
`scripts/phase7-external-mcp-contract.sh`.

catalogSha256: `dc3de5dd113c5a8c5aef9613f4bf3d45326bf025422a5e437a3e9a02bbb17092`

Tool names are public API identifiers after publication. Renames, removals,
new required properties, changed write/destructive semantics, coding-scope
changes, and risk-annotation changes require explicit review and republish.
Prefer additive optional properties and new tools.

After an approved change, run external MCP client Developer Mode **Refresh** or recreate/
republish where required, re-run **Scan Tools**, inspect schemas/descriptions/
annotations, and review action controls. New actions are not assumed enabled.
Business workspaces may require recreate/republish; Enterprise/Edu use Refresh
followed by action-control review. Stale clients cannot grant new capabilities;
server authorization and the Plan 028 sandbox remain authoritative.
