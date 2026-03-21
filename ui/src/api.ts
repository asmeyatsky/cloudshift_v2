import { TransformRequest, TransformResult } from './types'

function fetchWithTimeout(
  url: string,
  opts: RequestInit,
  timeoutMs = 30_000,
): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  return fetch(url, { ...opts, signal: controller.signal }).finally(() =>
    clearTimeout(timer),
  )
}

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

  const res = await fetchWithTimeout('/api/transform', {
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

export type GithubRepoFile = { path: string; source: string; language: string }

export type GithubRepoResponse = {
  files: GithubRepoFile[]
  resolved_ref?: string | null
  truncated: boolean
  error?: string | null
}

export async function importGithubRepo(
  url: string,
  opts?: { ref?: string; apiKey?: string },
): Promise<GithubRepoResponse> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }
  if (opts?.apiKey) {
    headers['X-API-Key'] = opts.apiKey.trim()
  }

  const res = await fetchWithTimeout('/api/github/repo', {
    method: 'POST',
    headers,
    body: JSON.stringify({
      url: url.trim(),
      ...(opts?.ref?.trim() ? { ref: opts.ref.trim() } : {}),
    }),
  }, 90_000)

  if (res.status === 401) {
    throw new Error('Unauthorized — add API key in Settings or use IAP')
  }
  if (res.status === 429) {
    throw new Error('Too many GitHub imports — try again in a minute')
  }
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
    const res = await fetchWithTimeout('/api/auth-check', { headers }, 5_000)
    if (!res.ok) return false
    const data = await res.json()
    return data.ok === true
  } catch {
    return false
  }
}
