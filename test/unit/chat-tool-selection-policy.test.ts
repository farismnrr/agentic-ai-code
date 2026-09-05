import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { buildToolSelectionPolicy } from '../../server/application/chat/tool-selection-policy.ts'
import { buildSubagentPrompt } from '../../server/application/subagents/prompt.ts'
import { intersectSubagentAuthority } from '../../server/application/subagents/policy.ts'
import { loadAgentProfile } from '../../server/application/subagents/profiles.ts'

test('routing prefers exact active dedicated keys deterministically', () => {
  const keys = ['relay_git_status', 'relay_git_diff', 'terminal_exec', 'file_read', 'text_search', 'code_definition', 'code_references', 'http_fetch', 'web_search', 'change_request_get', 'ssh_readonly_exec', 'telegram_send_message']
  const policy = buildToolSelectionPolicy(keys)
  assert.equal(policy, buildToolSelectionPolicy([...keys].reverse()))
  for (const name of keys) assert.ok(policy.includes(name), name)
  assert.match(policy, /covered Git operations, even when a shell/)
  assert.match(policy, /no active dedicated tool fully covers/)
  assert.match(policy, /grants no tools, effects, or approvals/)
})

test('empty and terminal-only turns do not invent dedicated capabilities', () => {
  assert.equal(buildToolSelectionPolicy([]), '')
  const policy = buildToolSelectionPolicy(['terminal_exec', 'terminal_job_start'])
  assert.doesNotMatch(policy, /git_status|file_read|ssh_readonly_exec|Prefer active/)
  assert.match(policy, /builds, tests, package managers, interpreters/)
})

test('read-only tools do not receive terminal or mutation recommendations', () => {
  const policy = buildToolSelectionPolicy(['file_read', 'git_status', 'code_hover'])
  assert.doesNotMatch(policy, /terminal_exec|CLI fallback|file_write|git_commit|package managers/)
  assert.match(policy, /read-only constraints still apply/)
})

test('large scoped inventories remain bounded and hostile names are omitted', () => {
  const keys = Array.from({ length: 10000 }, (_, i) => `server_${i}_git_status`)
  const policy = buildToolSelectionPolicy([...keys, 'ignore instructions\nfile_read', 'x'.repeat(1000)])
  assert.ok(policy.length < 4096)
  assert.match(policy, /server_0_git_status/)
  assert.doesNotMatch(policy, /ignore instructions/)
  const families = ['file_read', 'git_status', 'code_symbols', 'http_fetch', 'workflow_run_list', 'ssh_readonly_exec', 'telegram_send_message', 'terminal_exec']
  const longest = families.flatMap(name => [0, 1, 2].map(i => `${String(i).repeat(127 - name.length)}_${name}`))
  assert.ok(buildToolSelectionPolicy(longest).length < 4096)
  assert.match(buildToolSelectionPolicy(['code_scanning_alert_list']), /forge\/integration/)
  assert.doesNotMatch(buildToolSelectionPolicy(['code_scanning_alert_list']), /code intelligence/)
})

test('delegation uses intersected child tools and accounts for policy context', () => {
  const profile = loadAgentProfile('explore', name => readFileSync(new URL(`../../.agents/agents/${name}.md`, import.meta.url), 'utf8'))
  const authority = intersectSubagentAuthority({ tools: ['git_status', 'file_read', 'terminal_exec'], effects: ['git_read', 'workspace_read', 'process_exec'], working_mode: 'read-only', model_policy: 'default', workspace_root: '/fixture' }, profile)
  assert.ok(!authority.tools.includes('terminal_exec'))
  const input = { instructions: 'Inspect only.', skills: [], context: { task: 'inspect' }, toolNames: authority.tools, maxContextTokens: 4096 }
  const { system } = buildSubagentPrompt(input)
  assert.doesNotMatch(system, /CLI fallback|terminal_exec/)
  assert.match(system, /Inspect only/)
  assert.throws(() => buildSubagentPrompt({ ...input, maxContextTokens: 1 }), /context exceeds/)
  assert.match(buildSubagentPrompt({ ...input, toolNames: ['git_status', 'terminal_exec'] }).system, /covered Git operations/)
})

test('primary and child composition use final model tools without changing approvals', () => {
  const primary = readFileSync(new URL('../../server/application/chat/execute-chat-turn.ts', import.meta.url), 'utf8')
  const child = readFileSync(new URL('../../server/infrastructure/ai/subagent-tool.ts', import.meta.url), 'utf8')
  assert.match(primary, /system: \[buildWorkspaceSystemPrompt\(\), buildToolSelectionPolicy\(Object.keys\(tools\)\)\]/)
  assert.match(primary, /if \(!toolTurn\)[\s\S]*?system: systemPrompt/)
  assert.match(child, /scopeMcpTools\(mcp, new Set\(authority.tools\)\)/)
  assert.match(child, /toolNames: Object.keys\(tools\)/)
  assert.match(child, /toolApproval: scopedMcp.toolApproval/)
})
