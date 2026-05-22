import { apiFetch } from './client'

export const sendInvitation = (email: string) =>
  apiFetch<unknown>('/api/invitations', {
    method: 'POST',
    body: JSON.stringify({ email }),
  })

export const acceptInvite = (body: {
  token: string
  username: string
  first_name: string
  last_name: string
  password: string
}) =>
  apiFetch<unknown>('/api/auth/accept-invite', {
    method: 'POST',
    body: JSON.stringify(body),
  })
