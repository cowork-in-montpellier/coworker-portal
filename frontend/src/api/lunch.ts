import { z } from 'zod'
import { apiFetch } from './client'

export const WEEKDAYS = ['mon', 'tue', 'wed', 'thu', 'fri'] as const
const WeekdaySchema = z.enum(WEEKDAYS)
export type Weekday = z.infer<typeof WeekdaySchema>

export const WEEKDAY_LABELS: Record<Weekday, string> = {
  mon: 'Lu',
  tue: 'Ma',
  wed: 'Me',
  thu: 'Je',
  fri: 'Ve',
}

const GoerSchema = z.object({
  user_id: z.number(),
  first_name: z.string(),
  last_initial: z.string(),
})
export type Goer = z.infer<typeof GoerSchema>

const PlaceSchema = z.object({
  id: z.number(),
  name: z.string(),
  cuisine: z.string(),
  address: z.string(),
  maps_url: z.string().nullable(),
  open_days: z.array(WeekdaySchema),
  under_ten_euros: z.boolean(),
  veggie_friendly: z.boolean(),
  phone: z.string().nullable(),
  description: z.string(),
  is_gamelle: z.boolean(),
  open_today: z.boolean(),
  average_rating: z.number().nullable(),
  rating_count: z.number(),
  my_rating: z.number().nullable(),
  goers: z.array(GoerSchema),
})
export type Place = z.infer<typeof PlaceSchema>

const TodaySchema = z.object({
  date: z.string(),
  weekday: WeekdaySchema.nullable(),
  places: z.array(PlaceSchema),
  cuisines: z.array(z.string()),
  my_place_id: z.number().nullable(),
})
export type Today = z.infer<typeof TodaySchema>

export const PlaceInputSchema = z.object({
  name: z.string().trim().min(1, 'Donne un nom à cet endroit.').max(80, 'Le nom fait plus de 80 caractères.'),
  cuisine: z.string().trim().max(40, 'Le type fait plus de 40 caractères.'),
  address: z.string().trim().max(200, "L'adresse fait plus de 200 caractères."),
  maps_url: z
    .string()
    .trim()
    .refine((u) => u === '' || /^https?:\/\//.test(u), 'Le lien Google Maps doit commencer par https://')
    .transform((u) => (u === '' ? null : u)),
  open_days: z.array(WeekdaySchema).min(1, "Coche au moins un jour d'ouverture."),
  under_ten_euros: z.boolean(),
  veggie_friendly: z.boolean(),
  // Mirrors the backend: digits, spaces and `+ . - ( )`, at least 6 digits.
  phone: z
    .string()
    .trim()
    .refine(
      (p) => p === '' || (p.length <= 30 && /^[\d +.\-()]+$/.test(p) && p.replace(/\D/g, '').length >= 6),
      "Le numéro de téléphone n'est pas valide.",
    )
    .transform((p) => (p === '' ? null : p)),
  description: z.string().trim().max(500, 'La description fait plus de 500 caractères.'),
})
export type PlaceInput = z.input<typeof PlaceInputSchema>
type ValidPlaceInput = z.output<typeof PlaceInputSchema>

const IdSchema = z.object({ id: z.number() })

export async function fetchToday(): Promise<Today> {
  return TodaySchema.parse(await apiFetch('/api/lunch/today'))
}

export async function createPlace(place: ValidPlaceInput): Promise<number> {
  const res = IdSchema.parse(await apiFetch('/api/lunch/places', { method: 'POST', body: JSON.stringify(place) }))
  return res.id
}

export async function updatePlace(id: number, place: ValidPlaceInput): Promise<void> {
  await apiFetch(`/api/lunch/places/${id}`, { method: 'PUT', body: JSON.stringify(place) })
}

export async function deletePlace(id: number): Promise<void> {
  await apiFetch(`/api/lunch/places/${id}`, { method: 'DELETE' })
}

export async function setRating(id: number, stars: number | null): Promise<void> {
  await apiFetch(`/api/lunch/places/${id}/rating`, { method: 'PUT', body: JSON.stringify({ stars }) })
}

export async function setIntention(placeId: number | null): Promise<void> {
  await apiFetch('/api/lunch/today/intention', { method: 'PUT', body: JSON.stringify({ place_id: placeId }) })
}
