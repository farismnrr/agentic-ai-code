import { hasActivePairedDevice } from '../../infrastructure/database/devices'
import { buildLocalTerminalTool } from '../../infrastructure/ai/local-terminal-tool'

export async function createLocalTerminalPolicy({ userId, approvals, toolId }: { userId: string, approvals?: Record<string, 'always' | 'never'>, toolId: string }) {
  let paired = false
  try {
    paired = await hasActivePairedDevice(userId)
  } catch (err) {
    logger.error('[chat] failed to check paired relay-agent devices', err)
  }

  const approval = async (_input: { command: string, args?: string[] }) => {
    const decision = approvals?.[toolId]
    return decision === 'always' ? 'approved' : decision === 'never' ? 'denied' : 'user-approval'
  }

  // Tool construction (the `ai`/`@ai-code/terminal-tool` SDK surface) lives in
  // server/infrastructure/ai/local-terminal-tool.ts — the application layer
  // orchestrates pairing/approval policy only and never imports provider/AI
  // SDK packages directly (Plan 031A Phase 11).
  const tool = buildLocalTerminalTool()

  return { paired, tool, approval }
}
