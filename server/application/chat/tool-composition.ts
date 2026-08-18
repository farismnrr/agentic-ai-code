/** Compose parent-agent tools with first-party capabilities. Internal names win. */
export function composeAgentTools(internalTools: Record<string, unknown>, mcpTools: Record<string, unknown>) {
  const merged = { ...mcpTools }
  for (const [name, value] of Object.entries(internalTools)) merged[name] = value
  return merged
}
