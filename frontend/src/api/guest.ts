import { z } from 'zod'
import { ApiError, apiFetch } from './client'
import type { Service } from './services'

// ── Schemas ───────────────────────────────────────────────────────────────────

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

const GuestBillSchema = z.object({
  guest_token: z.string(),
  bill_id: z.number(),
  bill_number: z.string(),
  date: z.string(),
  amount: z.number(),
  is_paid: z.boolean(),
  payment_method: z.enum(['card', 'on_site']),
  checkout_failed: z.boolean(),
  lines: z.array(GuestBillLineSchema),
  payment_url: z.string().nullable(),
})

const GuestOrderSchema = z.object({
  order_token: z.string(),
  provisional_number: z.string(),
  payment_method: z.enum(['card', 'on_site']),
  total_amount: z.number(),
  checkout_failed: z.boolean(),
  guest_token: z.string().nullable(),
  payment_url: z.string().nullable(),
})

const GuestOrderPaymentStatusSchema = z.object({
  paid: z.boolean(),
  guest_token: z.string().nullable(),
})

// ── Types ─────────────────────────────────────────────────────────────────────

export type GuestBillResponse = z.infer<typeof GuestBillSchema>
export type GuestOrderData = z.infer<typeof GuestOrderSchema>
export type GuestOrderPaymentStatus = z.infer<typeof GuestOrderPaymentStatusSchema>

// ── Guest API (no auth required) ──────────────────────────────────────────────

async function guestFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
  })
  if (!res.ok) {
    let message = `API error ${res.status}`
    try {
      const body = await res.json()
      if (typeof body?.error === 'string') message = body.error
    } catch { /* non-JSON body */ }
    throw new ApiError(res.status, message)
  }
  return res.json() as Promise<T>
}

export async function listGuestServices(): Promise<Service[]> {
  const data = await guestFetch<{ data: Service[] }>('/api/guest/services')
  return data.data
}

// ── Orders (pending, pre-payment) ────────────────────────────────────────────

export interface CreateGuestOrderBody {
  lines: { service_id: number; quantity: number }[]
  guest_email: string
  billing_name?: string
  billing_address?: string
  payment_method?: 'card' | 'on_site'
}

export async function createGuestOrder(body: CreateGuestOrderBody): Promise<GuestOrderData> {
  const raw = await guestFetch<unknown>('/api/guest/orders', {
    method: 'POST',
    body: JSON.stringify(body),
  })
  return GuestOrderSchema.parse(raw)
}

export async function getGuestOrder(orderToken: string): Promise<GuestOrderData> {
  const raw = await guestFetch<unknown>(`/api/guest/orders/${orderToken}`)
  return GuestOrderSchema.parse(raw)
}

export async function getGuestOrderPaymentStatus(orderToken: string): Promise<GuestOrderPaymentStatus> {
  const raw = await guestFetch<unknown>(`/api/guest/orders/${orderToken}/payment-status`)
  return GuestOrderPaymentStatusSchema.parse(raw)
}

// ── Bills (post-payment) ──────────────────────────────────────────────────────

export async function getGuestBill(token: string): Promise<GuestBillResponse> {
  const raw = await guestFetch<unknown>(`/api/guest/bills/${token}`)
  return GuestBillSchema.parse(raw)
}

export async function checkGuestVouchers(token: string): Promise<Array<{ unify_id: string; code: string; duration: number; status: string }>> {
  const data = await guestFetch<{ data: Array<{ unify_id: string; code: string; duration: number; status: string }> }>(`/api/guest/bills/${token}/vouchers/check`)
  return data.data
}

// ── Cashier (authenticated) ───────────────────────────────────────────────────

export interface PendingOrderItem {
  order_token: string
  provisional_number: string
  total_amount: number
  created_at: string
  guest_email: string
}

export async function listPendingOrders(): Promise<PendingOrderItem[]> {
  const data = await apiFetch<{ data: PendingOrderItem[] }>('/api/guest/pending-orders')
  return data.data
}

export async function validateGuestOrder(orderToken: string): Promise<{ guest_token: string }> {
  return apiFetch<{ guest_token: string }>(`/api/guest/orders/${orderToken}/validate`, {
    method: 'POST',
  })
}

export async function downloadGuestBillPdf(token: string, billNumber: string): Promise<void> {
  const res = await fetch(`/api/guest/bills/${token}/pdf`)
  if (!res.ok) throw new ApiError(res.status, `PDF error ${res.status}`)
  const blob = await res.blob()
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `facture-${billNumber}.pdf`
  a.click()
  URL.revokeObjectURL(url)
}
