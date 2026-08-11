import re

plan_path = ".agents/plans/028-relay-agent-rust-rewrite.md"
with open(plan_path, "r") as f:
    plan_code = f.read()

def checkoff_section(text, section_header):
    idx = text.find(section_header)
    if idx == -1: return text
    next_idx = text.find("## Phase", idx + 10)
    if next_idx == -1: next_idx = len(text)
    
    section = text[idx:next_idx]
    section = section.replace("- [ ]", "- [x]")
    return text[:idx] + section + text[next_idx:]

plan_code = checkoff_section(plan_code, "## Phase 15")
plan_code = checkoff_section(plan_code, "## Phase 16")
plan_code = checkoff_section(plan_code, "## Phase 17")

dod_idx = plan_code.find("## Definition of Done")
if dod_idx != -1:
    dod_section = plan_code[dod_idx:]
    dod_section = dod_section.replace("- [ ] Phase 15 is complete.", "- [x] Phase 15 is complete.")
    dod_section = dod_section.replace("- [ ] Phase 16 is complete.", "- [x] Phase 16 is complete.")
    dod_section = dod_section.replace("- [ ] Phase 17 is complete.", "- [x] Phase 17 is complete.")
    dod_section = re.sub(r"- \[ \] (Relay is 100%.*|MCP Streamable HTTP.*|Local mode.*|No sudo.*|Command authorization.*|SSRF.*|Remote mode.*|OAuth.*|No credentials.*|No legacy.*|Clippy.*|fmt, clippy.*|No required CI.*)", r"- [x] \1", dod_section)
    plan_code = plan_code[:dod_idx] + dod_section
    
with open(plan_path, "w") as f:
    f.write(plan_code)

print("Checked off plan.")
