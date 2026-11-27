import { apiFetch } from './client'
import { z } from 'zod'

const OccupancySlotSchema = z.object({
  day: z.number(),   // 1=Mon … 5=Fri
  hour: z.number(),  // 8–20 (slot start hour, Paris time)
  count: z.number(),
})

const OccupancyResponseSchema = z.object({
  slots: z.array(OccupancySlotSchema),
  max_value: z.number(),
})

export type OccupancySlot = z.infer<typeof OccupancySlotSchema>
export type OccupancyResponse = z.infer<typeof OccupancyResponseSchema>

export async function fetchOccupancy(): Promise<OccupancyResponse> {
  const raw = await apiFetch<unknown>('/api/occupancy')
  return OccupancyResponseSchema.parse(raw)
}
