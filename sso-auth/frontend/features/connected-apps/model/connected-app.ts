export type ConnectedApp = {
  clientId: string
  name: string
  description: string
  callbackUrl: string
  enabled: boolean
  assertionTtlSeconds: number
}

export const defaultRelayApp = (): ConnectedApp => ({
  clientId: 'relay-agent',
  name: 'Masih Awam Relay',
  description: 'Agentic AI relay for Masih Awam.',
  callbackUrl: 'http://localhost:3100/connections/callback',
  enabled: true,
  assertionTtlSeconds: 90
})
