import { tool as aiTool, type Tool } from 'ai'
import { terminalToolSchema } from '@ai-code/terminal-tool'

const LOCAL_TERMINAL_DESCRIPTION = 'Execute a shell command on the user\'s own machine via their paired local CLI relay agent (a loopback bridge — this server never runs the command itself). Not scoped to any single project folder — pass an explicit `cwd` (absolute path) whenever the target directory matters, since it otherwise runs in the agent\'s own default directory, which may not be the folder the user means. Only available if the user has paired a device; if execution reports the agent is not connected, tell the user to open Settings → Local Terminal and pair it.'

/**
 * Builds the local-terminal `ai` SDK tool definition. This is the only place
 * `server/application/chat/local-terminal-policy.ts` reaches for tool
 * construction — the concrete `ai`/`@ai-code/terminal-tool` SDK surface stays
 * in server/infrastructure/ai/** (Plan 031A Phase 11).
 */
export function buildLocalTerminalTool(): Tool {
  return aiTool({
    description: LOCAL_TERMINAL_DESCRIPTION,
    inputSchema: terminalToolSchema
  })
}
