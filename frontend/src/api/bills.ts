import { z } from 'zod'
import { apiFetch } from './client'

// ── Schemas ──────────────────────────────────────────────────────────────────

const VoucherSchema = z.object({
  unify_id: z.string(),
  code: z.string(),
  duration: z.number(),
  status: z.string(),
})

const ManagedBillSchema = z.object({
  kind: z.literal('Managed'),
  id: z.number(),
  number: z.string(),
  date: z.string(),
  amount: z.number(),
  is_paid: z.boolean(),
  service_id: z.number(),
  vouchers: z.array(VoucherSchema),
})

const UnmanagedBillSchema = z.object({
  kind: z.literal('Unmanaged'),
  id: z.number(),
  number: z.string(),
  date: z.string(),
  amount: z.number(),
  is_paid: z.boolean(),
})

export const BillSchema = z.discriminatedUnion('kind', [ManagedBillSchema, UnmanagedBillSchema])
export type Bill = z.infer<typeof BillSchema>
export type ManagedBill = z.infer<typeof ManagedBillSchema>

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

export async function createBill(serviceId: number): Promise<Bill> {
  const raw = await apiFetch<unknown>('/api/bills', {
    method: 'POST',
    body: JSON.stringify({ service_id: serviceId }),
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
