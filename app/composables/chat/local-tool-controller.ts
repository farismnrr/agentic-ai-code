import { friendlyRelayErrorMessage } from '../../utils/chat-errors'

type ChatToolOutput = {
  addToolOutput: (value: {
    tool: string
    toolCallId: string
    options?: { headers?: Record<string, string> }
  } & ({ state: 'output-error', errorText: string } | { state?: 'output-available', output: unknown })) => void | PromiseLike<void>
}

export function createLocalToolController({ chat, relayAgent, ledger }: { chat: ChatToolOutput, relayAgent: { exec: (command: string, args: string[], cwd?: string) => Promise<unknown> }, ledger: ReturnType<typeof import('./attempt-ledger')['createAttemptLedger']> }) {
  const executed = new Set<string>()
  return async function runApproved(part: { toolCallId: string, input: unknown }) {
    if (executed.has(part.toolCallId) || ledger.hasAttempted(part.toolCallId)) return
    executed.add(part.toolCallId)
    ledger.markAttempted(part.toolCallId)
    const { command, args, cwd } = part.input as { command: string, args?: string[], cwd?: string }
    try {
      await chat.addToolOutput({ tool: 'local_terminal', toolCallId: part.toolCallId, output: await relayAgent.exec(command, args ?? [], cwd) })
    } catch (err: unknown) {
      await chat.addToolOutput({ tool: 'local_terminal', toolCallId: part.toolCallId, state: 'output-error', errorText: friendlyRelayErrorMessage(err) })
    }
  }
}
