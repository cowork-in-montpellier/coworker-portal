import { getRefreshToken, storeTokens, clearTokens } from '../auth'
import { renewToken } from './auth'

export class ApiError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

function authHeaders(): HeadersInit {
  const token = localStorage.getItem('token')
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  }
}

async function tryRefresh(): Promise<boolean> {
  const refreshToken = getRefreshToken()
  if (!refreshToken) return false
  try {
    const pair = await renewToken(refreshToken)
    storeTokens(pair.token, pair.refresh_token)
    return true
  } catch {
    clearTokens()
    return false
  }
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: { ...authHeaders(), ...(init?.headers ?? {}) },
  })

  if (res.status === 401) {
    const refreshed = await tryRefresh()
    if (refreshed) {
      const retry = await fetch(path, {
        ...init,
        headers: { ...authHeaders(), ...(init?.headers ?? {}) },
      })
      if (retry.status === 401) {
        clearTokens()
        window.location.href = '/login'
        throw new ApiError(401, 'Unauthorized')
      }
      if (!retry.ok) {
        let message = `API error ${retry.status}`
        try {
          const body = await retry.json()
          if (typeof body?.error === 'string') message = body.error
        } catch { /* non-JSON body, keep default */ }
        throw new ApiError(retry.status, message)
      }
      return retry.json() as Promise<T>
    }
    // Drop the rejected token, or /login sees it as still valid and sends us back here.
    clearTokens()
    window.location.href = '/login'
    throw new ApiError(401, 'Unauthorized')
  }

  if (!res.ok) {
    let message = `API error ${res.status}`
    try {
      const body = await res.json()
      if (typeof body?.error === 'string') message = body.error
    } catch { /* non-JSON body, keep default */ }
    throw new ApiError(res.status, message)
  }

  return res.json() as Promise<T>
}
