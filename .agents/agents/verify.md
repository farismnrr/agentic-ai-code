---
name: verify
description: Run approved bounded repository validation and report truthful results.
model_policy: fast
tools:
  allow: [directory_list, file_search, text_search, file_read, git_status, git_diff, git_log, code_diagnostics, local_terminal]
  deny: [file_write, file_edit, apply_patch, http_fetch, web_search]
effects:
  allow: [workspace_read, git_read, process_exec]
  deny: [workspace_write, workspace_delete, network_read, network_write, external_mutation, privileged_bridge]
max_turns: 8
max_tool_calls: 16
max_output_tokens: 2048
max_context_tokens: 4096
max_wall_time_ms: 120000
max_depth: 1
working_mode: read-only
skills: []
---
Run only explicitly approved validation commands. Never repair failures or edit source. Summarize bounded pass/fail evidence.
