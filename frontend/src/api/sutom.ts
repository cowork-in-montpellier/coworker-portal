import { z } from 'zod'

const TodaySchema = z.object({
  date: z.string(),
  word: z.string(),
  possible_words: z.array(z.string()),
})

export type Today = z.infer<typeof TodaySchema>

export async function fetchSutomToday(): Promise<Today> {
  const res = await fetch('/api/sutom/today')
  if (!res.ok) throw new Error(`API error ${res.status}`)
  return TodaySchema.parse(await res.json())
}
