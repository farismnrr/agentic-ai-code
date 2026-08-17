import type { LocalTerminalPort } from './contracts'
import type { RequestTelemetryContext } from '../observability/contracts'
import { approvalForCapability, capabilityFactsForToolCall } from '#shared/utils/capability-policy'

export async function createLocalTerminalPolicy({ userId, approvals, toolId, permissionMode = 'manual', localTerminal, telemetry }: { userId: string, approvals?: Record<string, 'always' | 'never'>, toolId: string, permissionMode?: 'plan' | 'workspace' | 'autonomous' | 'manual', localTerminal: LocalTerminalPort, telemetry?: RequestTelemetryContext }) {
  let paired = false
  try {
    paired = await localTerminal.hasPairedDevice(userId)
  } catch (err) {
    telemetry?.error('chat.local_terminal.pairing', 'pairing_lookup_failed', err)
  }

  const approval = async (input: { command: string, args?: string[] }) => {
    const decision = approvals?.[toolId]
    return approvalForCapability(capabilityFactsForToolCall({
      toolId,
      toolName: 'local_terminal',
      input,
      trustedProvenance: 'native'
    }), decision, permissionMode).outcome
  }

  // Tool construction (the `ai`/`@ai-code/terminal-tool` SDK surface) lives in
  // server/infrastructure/ai/local-terminal-tool.ts — the application layer
  // orchestrates pairing/approval policy only and never imports provider/AI
  // SDK packages directly (Plan 031A Phase 11).
  const tool = localTerminal.buildTool()

  return { paired, tool, approval }
}
