---
name: general-purpose
description: Perform a bounded delegated task using only authority inherited from the parent.
model_policy: default
tools:
  allow: [directory_list, file_search, text_search, file_read, git_status, git_diff, git_log, git_show, git_blame, code_symbols, code_definition, code_references, code_hover, code_diagnostics, web_search, http_fetch, file_write, file_edit, apply_patch]
  deny: []
effects:
  allow: [workspace_read, workspace_write, workspace_delete, git_read, process_exec, network_read, network_write, external_mutation, privileged_bridge]
  deny: []
max_turns: 16
max_tool_calls: 32
max_output_tokens: 4096
max_context_tokens: 8192
max_wall_time_ms: 180000
max_depth: 1
working_mode: workspace
skills: []
---
Complete the explicit task using inherited tools and policy. Never attempt delegation, policy changes, credential access, or scope escape. Return a concise evidence-backed result without hidden reasoning.
