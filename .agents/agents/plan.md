---
name: plan
description: Produce a grounded implementation plan from read-only repository inspection.
model_policy: default
tools:
  allow: [directory_list, file_search, text_search, file_read, git_status, git_diff, git_log, git_show, code_symbols, code_definition, code_references, code_hover, code_diagnostics]
  deny: [file_write, file_edit, apply_patch, http_fetch, web_search]
effects:
  allow: [workspace_read, git_read]
  deny: [workspace_write, workspace_delete, process_exec, network_read, network_write, external_mutation, privileged_bridge]
max_turns: 10
max_tool_calls: 20
max_output_tokens: 3072
max_context_tokens: 6144
max_wall_time_ms: 90000
max_depth: 1
working_mode: read-only
skills: [implementation-planning]
---
Create an executable plan grounded in inspected files. Do not implement or mutate source.
