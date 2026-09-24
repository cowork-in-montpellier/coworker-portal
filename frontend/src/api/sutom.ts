import { z } from 'zod'
import { apiFetch } from './client'

const LetterStatusSchema = z.enum(['correct', 'present', 'absent'])
export type LetterStatus = z.infer<typeof LetterStatusSchema>

const DaySchema = z.object({
  date: z.string(),
  puzzle_number: z.number(),
  word: z.string(),
  possible_words: z.array(z.string()),
  par: z.number().nullable(),
  par_average: z.number().nullable(),
  my_guesses: z.array(z.string()),
  my_score: z.number().nullable(),
})
export type Day = z.infer<typeof DaySchema>

const ScoreboardPlayerSchema = z.object({
  user_id: z.number(),
  first_name: z.string(),
  finished: z.boolean(),
  revealed: z.boolean(),
  score: z.number().nullable(),
  sequence: z.array(z.array(LetterStatusSchema)).nullable(),
  first_to_finish: z.boolean(),
  is_catchup: z.boolean(),
  points: z.number().nullable(),
})
export type ScoreboardPlayer = z.infer<typeof ScoreboardPlayerSchema>

const ScoreboardSchema = z.object({
  date: z.string(),
  players: z.array(ScoreboardPlayerSchema),
})
export type Scoreboard = z.infer<typeof ScoreboardSchema>

const LeaderboardEntrySchema = z.object({
  user_id: z.number(),
  first_name: z.string(),
  points: z.number(),
  games_played: z.number(),
  points_per_day: z.number(),
})
export type LeaderboardEntry = z.infer<typeof LeaderboardEntrySchema>

const LeaderboardSchema = z.object({
  since: z.string(),
  entries: z.array(LeaderboardEntrySchema),
})
export type Leaderboard = z.infer<typeof LeaderboardSchema>

const HistoryEntrySchema = z.object({
  date: z.string(),
  puzzle_number: z.number(),
  my_score: z.number().nullable(),
})
export type HistoryEntry = z.infer<typeof HistoryEntrySchema>

const HistorySchema = z.object({
  entries: z.array(HistoryEntrySchema),
})
export type History = z.infer<typeof HistorySchema>

const AttemptResultSchema = z.object({ score: z.number().nullable() })

export async function fetchToday(): Promise<Day> {
  return DaySchema.parse(await apiFetch('/api/sutom/today'))
}

export async function fetchDay(date: string): Promise<Day> {
  return DaySchema.parse(await apiFetch(`/api/sutom/day/${date}`))
}

export async function submitGuesses(date: string, guesses: string[]): Promise<{ score: number | null }> {
  return AttemptResultSchema.parse(
    await apiFetch(`/api/sutom/day/${date}/attempt`, {
      method: 'PUT',
      body: JSON.stringify({ guesses }),
    }),
  )
}

export async function fetchScoreboard(date: string): Promise<Scoreboard> {
  return ScoreboardSchema.parse(await apiFetch(`/api/sutom/day/${date}/scoreboard`))
}

export async function fetchLeaderboard(includeToday = true): Promise<Leaderboard> {
  return LeaderboardSchema.parse(await apiFetch(`/api/sutom/leaderboard?include_today=${includeToday}`))
}

export async function fetchHistory(): Promise<History> {
  return HistorySchema.parse(await apiFetch('/api/sutom/history'))
}
