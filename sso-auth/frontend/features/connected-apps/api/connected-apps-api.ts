import type { ConnectedApp } from '../model/connected-app'

export async function listConnectedApps(): Promise<ConnectedApp[]> {
  return await request<ConnectedApp[]>('/api/connected-apps')
}

export async function saveConnectedApp(app: ConnectedApp): Promise<ConnectedApp> {
  return await request<ConnectedApp>(
    `/api/connected-apps/${encodeURIComponent(app.clientId)}`,
    {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(app)
    }
  )
}

export async function deleteConnectedApp(clientId: string): Promise<void> {
  const response = await fetch(
    `/api/connected-apps/${encodeURIComponent(clientId)}`,
    {
      method: 'DELETE',
      headers: { Accept: 'application/json' },
      credentials: 'same-origin'
    }
  )
  if (!response.ok) {
    throw new Error(await responseMessage(response, 'Unable to disconnect app.'))
  }
}

async function request<T>(url: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers)
  headers.set('Accept', 'application/json')

  const response = await fetch(url, {
    ...init,
    headers,
    credentials: 'same-origin'
  })
  if (!response.ok) {
    throw new Error(await responseMessage(response, 'Connected app request failed.'))
  }
  return await response.json() as T
}

async function responseMessage(response: Response, fallback: string) {
  const message = (await response.text()).trim()
  return message || fallback
}
