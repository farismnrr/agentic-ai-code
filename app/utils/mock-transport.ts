import type { ChatTransport, UIMessage, UIMessageChunk } from 'ai'
import { simulateReadableStream } from 'ai'
import { pickScenario } from './fixtures/replies'

/**
 * A `ChatTransport` that answers from fixtures instead of a server.
 *
 * The transport is the seam the AI SDK provides for exactly this. Everything
 * above it — `useChat()`, its status machine, and every Nuxt UI chat component
 * — runs unmodified, so replacing this with
 * `new DefaultChatTransport({ api: '/api/chat' })` is the whole backend swap.
 *
 * `simulateReadableStream` is the SDK's own helper and is pull-based: when
 * `stop()` cancels the reader, pulls stop and the stream ends. That's why stop
 * works here without threading `abortSignal` through by hand.
 */
export interface MockChatTransportOptions {
  /** Delay before the first chunk, imitating time-to-first-token. */
  initialDelayInMs?: number
  /** Delay between chunks. */
  chunkDelayInMs?: number
  /** Tool ids enabled for the conversation, read at send time. */
  getEnabledToolIds?: () => string[]
  /**
   * Read at send time. When false the whole reply arrives at once, which is
   * what the "stream responses" setting turns off.
   */
  getStreaming?: () => boolean
}

export class MockChatTransport<UI_MESSAGE extends UIMessage = UIMessage>
implements ChatTransport<UI_MESSAGE> {
  private readonly initialDelayInMs: number
  private readonly chunkDelayInMs: number
  private readonly getEnabledToolIds: () => string[]
  private readonly getStreaming: () => boolean

  constructor(options: MockChatTransportOptions = {}) {
    this.initialDelayInMs = options.initialDelayInMs ?? 400
    this.chunkDelayInMs = options.chunkDelayInMs ?? 22
    this.getEnabledToolIds = options.getEnabledToolIds ?? (() => [])
    this.getStreaming = options.getStreaming ?? (() => true)
  }

  sendMessages({
    messages,
    trigger
  }: {
    trigger: 'submit-message' | 'regenerate-message'
    chatId: string
    messageId: string | undefined
    messages: UI_MESSAGE[]
    abortSignal: AbortSignal | undefined
  }): Promise<ReadableStream<UIMessageChunk>> {
    // On regenerate the last message is the assistant turn being replaced, so
    // walk back to the prompt that produced it.
    const lastUserMessage = [...messages].reverse().find(m => m.role === 'user')
    const prompt = lastUserMessage?.parts
      .map(part => (part.type === 'text' ? part.text : ''))
      .join('') ?? ''

    const scenario = pickScenario(prompt)
    const body = scenario.build({ enabledToolIds: this.getEnabledToolIds() })

    const chunks: UIMessageChunk[] = [
      { type: 'start' },
      { type: 'start-step' },
      ...body,
      { type: 'finish-step' },
      { type: 'finish', finishReason: scenario.id === 'error' ? 'error' : 'stop' }
    ]

    const streaming = this.getStreaming()

    return Promise.resolve(
      simulateReadableStream({
        chunks,
        // Regeneration shouldn't re-pay the full thinking pause.
        initialDelayInMs: streaming
          ? (trigger === 'regenerate-message' ? 150 : this.initialDelayInMs)
          : 0,
        chunkDelayInMs: streaming ? this.chunkDelayInMs : 0
      })
    )
  }

  /** No server means no stream to resume. */
  reconnectToStream(): Promise<ReadableStream<UIMessageChunk> | null> {
    return Promise.resolve(null)
  }
}
