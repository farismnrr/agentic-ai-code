import { tool as aiTool, type Tool } from 'ai'
import { z } from 'zod'

const LOCAL_TERMINAL_DESCRIPTION = 'Execute a shell command on the user\'s own machine through the browser-local CLI relay (a loopback bridge — this server never runs the command itself). Not scoped to any single project folder — pass an explicit `cwd` (absolute path) whenever the target directory matters, since it otherwise runs in the relay\'s default directory. Choose `timeout_ms` based on the expected command duration. Prefer `execution_mode=sync` for short operations whose result is needed immediately, `async` for long-running builds/tests/installations, and `auto` when either is acceptable. Async work is task-backed and continues independently of the initiating HTTP round trip. The capability is available only while the local relay is enabled and connected; if it is disconnected, direct the user to Settings → MCP → Local relay.'

const localTerminalToolSchema = z.object({
  command: z.string().describe('The binary to run, e.g. "ls" — do not include flags/arguments here, put them in `args` instead.'),
  args: z.array(z.string()).optional().describe('Arguments for the command, e.g. ["-la", "."].'),
  cwd: z.string().optional().describe('Absolute path to run the command in. Omit to use the relay agent default directory.'),
  timeout_ms: z.number().int().min(0).optional().describe('Requested command runtime in milliseconds. Pick a realistic value for the operation; 0 means no command deadline unless the relay operator configured a maximum.'),
  execution_mode: z.enum(['sync', 'async', 'auto']).optional().default('auto').describe('Execution strategy. Use sync for short commands, async for long-running work that should survive the initial request, or auto to let the relay use task execution when supported and safe.')
})

/**
 * Builds the local-terminal `ai` SDK tool definition. This is the only place
 * `server/application/chat/local-terminal-policy.ts` reaches for tool
 * construction — the concrete `ai`/`@ai-code/terminal-tool` SDK surface stays
 * in server/infrastructure/ai/** (Plan 031A Phase 11).
 */
export function buildLocalTerminalTool(): Tool {
  return aiTool({
    description: LOCAL_TERMINAL_DESCRIPTION,
    inputSchema: localTerminalToolSchema
  })
}
