import type { LocalTerminalPort } from './contracts'
import { approvalForCapability, capabilityFactsForToolCall } from '#shared/utils/capability-policy'

export function createLocalTerminalPolicy({ approvals, toolId, permissionMode = 'manual', localTerminal }: { approvals?: Record<string, 'always' | 'never'>, toolId: string, permissionMode?: 'plan' | 'bypass' | 'manual', localTerminal: LocalTerminalPort }) {
  const approval = async (input: { command: string, args?: string[] }) => {
    const decision = approvals?.[toolId]
    return approvalForCapability(capabilityFactsForToolCall({
      toolId,
      toolName: 'local_terminal',
      input,
      trustedProvenance: 'native'
    }), decision, permissionMode).outcome
  }

  // Availability is controlled by enabledToolIds + the live client-side relay
  // connection. The server only defines the client-executed tool and its
  // permission policy; it no longer adds a hidden pairing prerequisite.
  return { tool: localTerminal.buildTool(), approval }
}
