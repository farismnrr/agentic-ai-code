export type ConnectedApp = {
  clientId: string
  name: string
  description: string
  callbackUrl: string
  enabled: boolean
  assertionTtlSeconds: number
}

export const emptyConnectedApp = (): ConnectedApp => ({
  clientId: '',
  name: '',
  description: '',
  callbackUrl: '',
  enabled: false,
  assertionTtlSeconds: 0
})
