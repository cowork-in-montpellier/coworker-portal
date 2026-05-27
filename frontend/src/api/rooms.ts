import { z } from 'zod'
import { apiFetch } from './client'

export const RoomSchema = z.object({
  id: z.number(),
  name: z.string(),
  color: z.string(),
})
export type Room = z.infer<typeof RoomSchema>

export const BookingSchema = z.object({
  id: z.number(),
  room_id: z.number(),
  title: z.string(),
  start_at: z.string(),
  end_at: z.string(),
  created_by: z.number(),
  notes: z.string(),
  created_at: z.string(),
})
export type Booking = z.infer<typeof BookingSchema>

export const listRooms = () =>
  apiFetch<Room[]>('/api/rooms').then(data => z.array(RoomSchema).parse(data))

export const listBookings = (start: Date, end: Date) =>
  apiFetch<Booking[]>(
    `/api/bookings?start=${encodeURIComponent(start.toISOString())}&end=${encodeURIComponent(end.toISOString())}`,
  ).then(data => z.array(BookingSchema).parse(data))

export const createBooking = (body: {
  room_id: number
  title: string
  start_at: string
  end_at: string
  notes?: string
}) =>
  apiFetch<Booking>('/api/bookings', {
    method: 'POST',
    body: JSON.stringify(body),
  }).then(data => BookingSchema.parse(data))

export const deleteBooking = (id: number) =>
  apiFetch<unknown>(`/api/bookings/${id}`, { method: 'DELETE' })
