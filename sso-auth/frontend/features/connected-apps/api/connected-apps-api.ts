import type { ConnectedApp } from '../model/connected-app'

export async function listConnectedApps(): Promise<ConnectedApp[]> {
  const response = await fetch('/api/connected-apps', {
    headers: { Accept: 'application/json' },
    credentials: 'same-origin'
  })
  if (!response.ok) {
    throw new Error(await responseMessage(response, 'Unable to load connected apps.'))
  }
  return await response.json() as ConnectedApp[]
}

export async function saveConnectedApp(app: ConnectedApp): Promise<ConnectedApp> {
  const response = await fetch(
    `/api/connected-apps/${encodeURIComponent(app.clientId)}`,
    {
      method: 'PUT',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json'
      },
      credentials: 'same-origin',
      body: JSON.stringify(app)
    }
  )
  if (!response.ok) {
    throw new Error(await responseMessage(response, 'Unable to save connected app.'))
  }
  return await response.json() as ConnectedApp
}

async function responseMessage(response: Response, fallback: string) {
  const message = (await response.text()).trim()
  return message || fallback
}
