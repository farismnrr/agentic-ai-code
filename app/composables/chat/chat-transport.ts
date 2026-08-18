import { DefaultChatTransport } from 'ai'
import type { UIMessage } from '#shared/types/chat'

// Plan 035 P1 — "actual browser chat trace continuity". `DefaultChatTransport`
// uses the AI SDK's own internal fetch resolution (see
// `HttpChatTransportInitOptions.fetch` in node_modules/ai's type
// definitions), NOT `globalThis.$fetch` — so the `trace-context.client.ts`
// plugin's override of `globalThis.$fetch` never touched chat requests at
// all. Passing the explicit `fetch` option here (rather than monkeypatching
// the native global `fetch`) wires chat traffic through the SAME
// `createTracedFetch()` primitive the global `$fetch` override uses, so
// trace generation/telemetry logic is never duplicated between the two.
export function createConversationTransport(agentContext: Ref<{ repository_identity?: string } | undefined>, agentSessionReady: Ref<boolean>) {
  const telemetry = useTelemetry()

  return new DefaultChatTransport({
    api: '/api/chat',
    fetch: createTracedFetch(telemetry, globalThis.fetch),
    prepareSendMessagesRequest: ({ id, messages, trigger, messageId }) => ({
      ...(agentSessionReady.value ? {} : (() => { throw new Error('Agent security session is not ready') })()),
      body: {
        id,
        trigger,
        messageId,
        message: messages[messages.length - 1] as UIMessage | undefined,
        agentContext: agentContext.value
      }
    })
  })
}
