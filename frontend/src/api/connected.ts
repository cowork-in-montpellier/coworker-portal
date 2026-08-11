import { z } from 'zod'
import { apiFetch } from './client'

const ConnectedAccountUserSchema = z.object({
  user_id: z.number(),
  username: z.string(),
  first_name: z.string(),
  voucher_unify_id: z.string(),
  macs: z.array(z.string()),
  minutes_remaining: z.number().nullable(),
})

const ConnectedGuestsResponseSchema = z.object({
  total: z.number(),
  account_users: z.array(ConnectedAccountUserSchema),
  guest_count: z.number(),
  unknown_count: z.number(),
})

export type ConnectedAccountUser = z.infer<typeof ConnectedAccountUserSchema>
export type ConnectedGuestsResponse = z.infer<typeof ConnectedGuestsResponseSchema>

export async function fetchConnected(): Promise<ConnectedGuestsResponse> {
  const raw = await apiFetch<unknown>('/api/connected')
  return ConnectedGuestsResponseSchema.parse(raw)
}
