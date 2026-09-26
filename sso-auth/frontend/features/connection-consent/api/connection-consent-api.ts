export type ConnectionAppInfo = {
  clientId: string
  name: string
  description: string
}

export async function loadConnectionApp(clientId: string): Promise<ConnectionAppInfo> {
  const response = await fetch(
    `/api/connect/apps/${encodeURIComponent(clientId)}`,
    {
      headers: { Accept: 'application/json' },
      credentials: 'same-origin'
    }
  )
  if (!response.ok) {
    const message = (await response.text()).trim()
    throw new Error(message || 'This connected app is unavailable.')
  }
  return await response.json() as ConnectionAppInfo
}
