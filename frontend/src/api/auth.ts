import { apiFetch } from './client'

export const forgotPassword = (email: string) =>
  apiFetch<unknown>('/api/auth/forgot-password', {
    method: 'POST',
    body: JSON.stringify({ email }),
  })

export const resetPassword = (token: string, new_password: string) =>
  apiFetch<unknown>('/api/auth/reset-password', {
    method: 'POST',
    body: JSON.stringify({ token, new_password }),
  })

export interface TokenPair {
  token: string
  refresh_token: string
}

export async function renewToken(refreshToken: string): Promise<TokenPair> {
  const res = await fetch('/api/auth/renew', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  })
  if (!res.ok) throw new Error('Renew failed')
  return res.json() as Promise<TokenPair>
}
