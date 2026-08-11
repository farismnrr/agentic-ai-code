import re

plan_path = ".agents/plans/028-relay-agent-rust-rewrite.md"
with open(plan_path, "r") as f:
    plan_code = f.read()

plan_code = plan_code.replace("- [ ] Remote `tools/call` requires valid OAuth authorization before any side effect.", "- [x] Remote `tools/call` requires valid OAuth authorization before any side effect.")
plan_code = plan_code.replace("- [ ] Scope authorization must be enforced independently of tool arguments.", "- [x] Scope authorization must be enforced independently of tool arguments.")

with open(plan_path, "w") as f:
    f.write(plan_code)

print("Checked off MCP contract.")
