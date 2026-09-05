/** Routing advice is derived from the final model-facing keys, never authority. */
export function buildToolSelectionPolicy(toolNames: Iterable<string>): string {
  const names = [...new Set(toolNames)].filter(name => /^[a-zA-Z0-9_.-]{1,128}$/.test(name)).sort()
  if (!names.length) return ''
  const groups: [string, RegExp][] = [
    ['structured filesystem/search', /(?:^|[_.])(?:directory_list|file_(?:search|read|write|edit)|text_search|apply_patch)$/],
    ['structured Git', /(?:^|[_.])git_[a-z_]+$/],
    ['code intelligence', /(?:^|[_.])code_(?:symbols|definition|references|implementations|hover|diagnostics|rename_preview)$/],
    ['HTTP/web', /(?:^|[_.])(?:http_fetch|web_search)$/],
    ['forge/integration', /(?:^|[_.])(?:change_request|issue|workflow|actions|security|forge|dependabot_alert|code_scanning_alert|secret_scanning_alert)_[a-z_]+$/],
    ['remote diagnostics', /(?:^|[_.])ssh_readonly_exec$/],
    ['messaging', /(?:^|[_.])telegram_send_message$/]
  ]
  const lines = ['Tool selection: inspect the supplied tool schemas. Prefer a dedicated MCP tool when it fully covers the requested operation. This advice grants no tools, effects, or approvals; all permission and read-only constraints still apply.']
  for (const [label, pattern] of groups) {
    const active = names.filter(name => pattern.test(name))
    if (!active.length) continue
    // Examples stay bounded even when a remote server exposes a large catalog.
    lines.push(`Prefer active ${label} tools: ${active.slice(0, 2).join(', ')}${active.length > 2 ? ' (and other supplied tools in this family)' : ''}.`)
    if (label === 'structured Git') lines.push('Use these tools for covered Git operations, even when a shell could perform the same operation.')
  }
  const terminals = names.filter(name => /(?:^|[_.])terminal_(?:exec|job_start)$/.test(name))
  if (terminals.length) {
    lines.push(`CLI fallback: ${terminals.slice(0, 2).join(', ')}. Use for builds, tests, package managers, interpreters, project scripts, sandbox-local process/service commands, composite shell workflows, or operations no active dedicated tool fully covers. Covered Git/file/network operations should use their dedicated tools first. Do not request an extra tool-discovery call; the supplied schemas are the active inventory.`)
  }
  return lines.join('\n')
}
