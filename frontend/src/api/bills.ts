import { z } from 'zod'
import { apiFetch } from './client'

// ── Schemas ──────────────────────────────────────────────────────────────────

const VoucherSchema = z.object({
  unify_id: z.string(),
  code: z.string(),
  duration: z.number(),
  status: z.string(),
  active_days_count: z.number(),
})

const BillLineSchema = z.object({
  id: z.number(),
  service_id: z.number().nullable(),
  quantity: z.number(),
  vouchers: z.array(VoucherSchema),
})

export const BillSchema = z.object({
  id: z.number(),
  number: z.string(),
  date: z.string(),
  amount: z.number(),
  is_paid: z.boolean(),
  lines: z.array(BillLineSchema),
})

export type Bill = z.infer<typeof BillSchema>
export type BillLine = z.infer<typeof BillLineSchema>
export type Voucher = z.infer<typeof VoucherSchema>

const ListBillsResponseSchema = z.object({
  total: z.number(),
  data: z.array(BillSchema),
})

export type ListBillsResponse = z.infer<typeof ListBillsResponseSchema>

// ── API calls ─────────────────────────────────────────────────────────────────

export interface BillsQuery {
  offset?: number
  limit?: number
  number?: string
  date_from?: string
  date_to?: string
}

export async function createBill(lines: { service_id: number; quantity: number }[]): Promise<Bill> {
  const raw = await apiFetch<unknown>('/api/bills', {
    method: 'POST',
    body: JSON.stringify({ lines }),
  })
  return BillSchema.parse(raw)
}

const VoucherStatusResponseSchema = z.object({
  unify_id: z.string(),
  code: z.string(),
  duration: z.number(),
  status: z.string(),
})

const VoucherCheckResponseSchema = z.object({
  data: z.array(VoucherStatusResponseSchema),
})

export type VoucherStatusEntry = z.infer<typeof VoucherStatusResponseSchema>

export async function checkVouchers(billId: number): Promise<VoucherStatusEntry[]> {
  const raw = await apiFetch<unknown>(`/api/bills/${billId}/vouchers/check`)
  return VoucherCheckResponseSchema.parse(raw).data
}

export const revokeVoucher = (billId: number, unifyId: string) =>
  apiFetch<unknown>(`/api/bills/${billId}/vouchers/${unifyId}/revoke`, { method: 'POST' })

export async function downloadBillPdf(billId: number, billNumber: string): Promise<void> {
  const res = await fetch(`/api/bills/${billId}/pdf`, {
    headers: { Authorization: `Bearer ${localStorage.getItem('token') ?? ''}` },
  })
  if (!res.ok) throw new Error(`Erreur ${res.status}`)
  const blob = await res.blob()
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${billNumber}.pdf`
  a.click()
  URL.revokeObjectURL(url)
}


export async function listBills(query: BillsQuery = {}): Promise<ListBillsResponse> {
  const params = new URLSearchParams()
  if (query.offset !== undefined) params.set('offset', String(query.offset))
  if (query.limit !== undefined) params.set('limit', String(query.limit))
  if (query.number) params.set('number', query.number)
  if (query.date_from) params.set('date_from', query.date_from)
  if (query.date_to) params.set('date_to', query.date_to)

  const qs = params.size > 0 ? `?${params}` : ''
  const raw = await apiFetch<unknown>(`/api/bills${qs}`)
  return ListBillsResponseSchema.parse(raw)
}
