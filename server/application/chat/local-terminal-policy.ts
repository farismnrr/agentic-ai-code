import { and, eq, isNull } from 'drizzle-orm'
import { userDevices } from '../../database/schema'
import { tool as aiTool, type Tool } from 'ai'
import { terminalToolSchema } from '@ai-code/terminal-tool'

const LOCAL_TERMINAL_DESCRIPTION = 'Execute a shell command on the user\'s own machine via their paired local CLI relay agent (a loopback bridge — this server never runs the command itself). Not scoped to any single project folder — pass an explicit `cwd` (absolute path) whenever the target directory matters, since it otherwise runs in the agent\'s own default directory, which may not be the folder the user means. Only available if the user has paired a device; if execution reports the agent is not connected, tell the user to open Settings → Local Terminal and pair it.'

type Db = ReturnType<typeof useDb>

export async function createLocalTerminalPolicy({ db, userId, approvals, toolId }: { db: Db, userId: string, approvals?: Record<string, 'always' | 'never'>, toolId: string }) {
  let paired = false
  try {
    const [device] = await db.select({ id: userDevices.id })
      .from(userDevices)
      .where(and(eq(userDevices.userId, userId), isNull(userDevices.revokedAt)))
      .limit(1)
    paired = Boolean(device)
  } catch (err) {
    logger.error('[chat] failed to check paired relay-agent devices', err)
  }

  const approval = async (_input: { command: string, args?: string[] }) => {
    const decision = approvals?.[toolId]
    return decision === 'always' ? 'approved' : decision === 'never' ? 'denied' : 'user-approval'
  }

  const tool: Tool = aiTool({
    description: LOCAL_TERMINAL_DESCRIPTION,
    inputSchema: terminalToolSchema
  })

  return { paired, tool, approval }
}
