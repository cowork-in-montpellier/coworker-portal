import { z } from 'zod'
import { apiFetch } from './client'

const ProfileSchema = z.object({
  id: z.number(),
  username: z.string(),
  first_name: z.string(),
  last_name: z.string(),
  email: z.string(),
  billing_address: z.string(),
  onsite_payment: z.boolean(),
})

export type Profile = z.infer<typeof ProfileSchema>

export const getProfile = () =>
  apiFetch<unknown>('/api/profile').then(d => ProfileSchema.parse(d))

export const updateProfile = (body: {
  first_name: string
  last_name: string
  email: string
  billing_address: string
  onsite_payment?: boolean
}) =>
  apiFetch<unknown>('/api/profile', {
    method: 'PUT',
    body: JSON.stringify(body),
  }).then(d => ProfileSchema.parse(d))

export const changePassword = (body: {
  current_password: string
  new_password: string
}) =>
  apiFetch<unknown>('/api/profile/password', {
    method: 'PUT',
    body: JSON.stringify(body),
  })
