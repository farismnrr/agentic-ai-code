import { friendlyRelayErrorMessage } from '../../utils/chat-errors'

type ChatToolOutput = {
  addToolOutput: (value: {
    tool: string
    toolCallId: string
    options?: { headers?: Record<string, string> }
  } & ({ state: 'output-error', errorText: string } | { state?: 'output-available', output: unknown })) => void | PromiseLike<void>
}

export function createLocalToolController({ chat, relayAgent, ledger, agentSession, requestApproval }: { chat: ChatToolOutput, relayAgent: { exec: (command: string, args?: string[], cwd?: string, agentSession?: string, hookApprovalToken?: string) => Promise<{ approvalRequired?: boolean, approvalToken?: string, error?: string }> }, ledger: ReturnType<typeof import('./attempt-ledger')['createAttemptLedger']>, agentSession?: string, requestApproval: (input: unknown, token: string) => Promise<boolean> }) {
  const executed = new Set<string>()
  return async function runApproved(part: { toolCallId: string, input: unknown }) {
    if (executed.has(part.toolCallId) || ledger.hasAttempted(part.toolCallId)) return
    executed.add(part.toolCallId)
    ledger.markAttempted(part.toolCallId)
    const { command, args, cwd } = part.input as { command: string, args?: string[], cwd?: string }
    try {
      let result = await relayAgent.exec(command, args ?? [], cwd, agentSession)
      if (result.approvalRequired && result.approvalToken) {
        const approved = await requestApproval(part.input, result.approvalToken)
        if (approved) result = await relayAgent.exec(command, args ?? [], cwd, agentSession, result.approvalToken)
        else result = { ...result, error: 'Hook approval denied' }
      }
      await chat.addToolOutput({
        tool: 'local_terminal',
        toolCallId: part.toolCallId,
        output: result.approvalRequired
          ? { type: 'approval_required', reason: result.error }
          : result
      })
    } catch (err: unknown) {
      await chat.addToolOutput({ tool: 'local_terminal', toolCallId: part.toolCallId, state: 'output-error', errorText: friendlyRelayErrorMessage(err) })
    }
  }
}
