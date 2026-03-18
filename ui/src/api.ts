import { TransformRequest, TransformResult } from './types'

export async function transform(
  req: TransformRequest,
  apiKey?: string,
): Promise<TransformResult> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }
  if (apiKey) {
    headers['X-API-Key'] = apiKey.trim()
  }

  const res = await fetch('/api/transform', {
    method: 'POST',
    headers,
    body: JSON.stringify(req),
  })

  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || `HTTP ${res.status}`)
  }

  return res.json()
}

export async function checkAuth(apiKey?: string): Promise<boolean> {
  const headers: Record<string, string> = {}
  if (apiKey) {
    headers['X-API-Key'] = apiKey.trim()
  }

  try {
    const res = await fetch('/api/auth-check', { headers })
    if (!res.ok) return false
    const data = await res.json()
    return data.ok === true
  } catch {
    return false
  }
}
