import { z } from 'zod'
import { ApiError } from './client'
import { ServiceSchema } from './services'
import type { VoucherStatusEntry } from './bills'

// ── Schemas ──────────────────────────────────────────────────────────────────

const GuestServicesResponseSchema = z.object({
  data: z.array(ServiceSchema),
})

const GuestVoucherSchema = z.object({
  unify_id: z.string(),
  code: z.string(),
  duration: z.number(),
  status: z.string(),
  active_days_count: z.number(),
})

const GuestBillLineSchema = z.object({
  service_name: z.string(),
  quantity: z.number(),
  vouchers: z.array(GuestVoucherSchema),
})

export const GuestBillResponseSchema = z.object({
  guest_token: z.string(),
  bill_id: z.number(),
  bill_number: z.string(),
  date: z.string(),
  amount: z.number(),
  is_paid: z.boolean(),
  lines: z.array(GuestBillLineSchema),
  payment_url: z.string().nullable().optional(),
})

export type GuestBillResponse = z.infer<typeof GuestBillResponseSchema>

const VoucherCheckResponseSchema = z.object({
  data: z.array(z.object({
    unify_id: z.string(),
    code: z.string(),
    duration: z.number(),
    status: z.string(),
  })),
})

// ── API calls ─────────────────────────────────────────────────────────────────

async function guestFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
  })
  if (!res.ok) throw new ApiError(res.status, `API error ${res.status}`)
  return res.json() as Promise<T>
}

export async function listGuestServices() {
  const raw = await guestFetch<unknown>('/api/guest/services')
  return GuestServicesResponseSchema.parse(raw).data
}

export interface CreateGuestBillRequest {
  lines: { service_id: number; quantity: number }[]
  billing_name?: string
  billing_address?: string
  payment_method?: string
}

export async function createGuestBill(body: CreateGuestBillRequest): Promise<GuestBillResponse> {
  const raw = await guestFetch<unknown>('/api/guest/bills', {
    method: 'POST',
    body: JSON.stringify(body),
  })
  return GuestBillResponseSchema.parse(raw)
}

export async function getGuestBill(token: string): Promise<GuestBillResponse> {
  const raw = await guestFetch<unknown>(`/api/guest/bills/${token}`)
  return GuestBillResponseSchema.parse(raw)
}

export async function checkGuestVouchers(token: string): Promise<VoucherStatusEntry[]> {
  const raw = await guestFetch<unknown>(`/api/guest/bills/${token}/vouchers/check`)
  return VoucherCheckResponseSchema.parse(raw).data
}

export async function downloadGuestBillPdf(token: string, billNumber: string): Promise<void> {
  const res = await fetch(`/api/guest/bills/${token}/pdf`)
  if (!res.ok) throw new Error(`Erreur ${res.status}`)
  const blob = await res.blob()
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${billNumber}.pdf`
  a.click()
  URL.revokeObjectURL(url)
}
