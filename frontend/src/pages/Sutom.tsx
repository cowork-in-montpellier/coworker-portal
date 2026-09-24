import { addDays, format } from 'date-fns'
import { fr } from 'date-fns/locale'
import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import {
  fetchDay,
  fetchHistory,
  fetchLeaderboard,
  fetchScoreboard,
  fetchToday,
  submitGuesses,
  type Day,
  type History,
  type Leaderboard,
  type LetterStatus,
  type Scoreboard,
  type ScoreboardPlayer,
} from '../api/sutom'
import { ApiError } from '../api/client'
import { getTokenPayload } from '../auth'
import { Navbar } from '../components/Navbar'
import { useToast } from '../lib/toast'

const MAX_ATTEMPTS = 6
const FAILED_SCORE = MAX_ATTEMPTS + 1
// Mirrors the backend's HISTORY_WINDOW_DAYS: how far back a previous day can be replayed.
const HISTORY_WINDOW_DAYS = 30
// Delay between each letter's color reveal on a freshly submitted guess.
const REVEAL_DELAY_MS = 300
// Mirrors the backend's leaderboard_start_date(): days before this were warm-up/training
// and never count toward leaderboard points.
const LEADERBOARD_START_DATE = '2026-09-23'

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

interface LetterResult {
  letter: string
  status: LetterStatus
}

// Mirrors the source site's `analyserMot`: exact matches are resolved first, then
// leftover letter counts are handed out left-to-right to misplaced-letter guesses.
function scoreGuess(target: string, guess: string): LetterResult[] {
  const targetLetters = target.split('')
  const guessLetters = guess.split('')
  const remaining: Record<string, number> = {}

  targetLetters.forEach((letter, i) => {
    if (guessLetters[i] !== letter) remaining[letter] = (remaining[letter] ?? 0) + 1
  })

  return guessLetters.map((letter, i) => {
    if (targetLetters[i] === letter) return { letter, status: 'correct' as const }
    if ((remaining[letter] ?? 0) > 0) {
      remaining[letter]--
      return { letter, status: 'present' as const }
    }
    return { letter, status: 'absent' as const }
  })
}

const STATUS_PRIORITY: Record<LetterStatus, number> = {
  absent: 0,
  present: 1,
  correct: 2,
}

function keyboardStatuses(target: string, guesses: string[]): Record<string, LetterStatus> {
  const statuses: Record<string, LetterStatus> = {}
  guesses.forEach((guess) => {
    scoreGuess(target, guess).forEach(({ letter, status }) => {
      const current = statuses[letter]
      if (!current || STATUS_PRIORITY[status] > STATUS_PRIORITY[current]) statuses[letter] = status
    })
  })
  return statuses
}

const KEYBOARD_ROWS = [
  ['A', 'Z', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'],
  ['Q', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'M'],
  ['W', 'X', 'C', 'V', 'B', 'N', 'BACKSPACE', '.', 'ENTER'],
]

// The original SUTOM's own palette: unfilled/not-found letters stay the grid's blue,
// well-placed letters turn the whole tile red, and misplaced letters get a yellow
// circle rather than a full-tile color (source: sutom.nocle.fr's jeu.css).
const COLOR_UNKNOWN = '#0077c7'
const COLOR_CORRECT = '#e7002a'
const COLOR_PRESENT = '#ffbd00'
const COLOR_ABSENT_KEY = '#707070'

function miniCellClass(status: LetterStatus): string {
  switch (status) {
    case 'correct':
      return 'bg-[#e7002a]'
    case 'present':
      return 'bg-[#ffbd00]'
    case 'absent':
      return 'bg-[#0077c7]'
  }
}

// Tailwind's scanner can't see colors interpolated at runtime, so the colored key
// states are applied as inline styles instead of dynamic class names.
function keyStyle(status: LetterStatus | undefined): CSSProperties | undefined {
  switch (status) {
    case 'correct':
      return {
        backgroundColor: COLOR_CORRECT,
        borderColor: COLOR_CORRECT,
        color: 'white',
      }
    case 'present':
      return {
        backgroundColor: COLOR_PRESENT,
        borderColor: COLOR_PRESENT,
        color: 'white',
      }
    case 'absent':
      return { borderColor: COLOR_ABSENT_KEY, color: COLOR_ABSENT_KEY }
    default:
      return undefined
  }
}

// Single/double chevrons as SVG paths rather than text glyphs, so they sit dead-center
// in a circular button regardless of font metrics.
const CHEVRON_LEFT = 'M15.75 19.5 8.25 12l7.5-7.5'
const CHEVRON_RIGHT = 'M8.25 4.5l7.5 7.5-7.5 7.5'
const CHEVRON_DOUBLE_LEFT = 'M18.75 19.5l-7.5-7.5 7.5-7.5m-6 15L5.25 12l7.5-7.5'
const CHEVRON_DOUBLE_RIGHT = 'M11.25 4.5l7.5 7.5-7.5 7.5m-6-15l7.5 7.5-7.5 7.5'

function ChevronIcon({ path }: { path: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" d={path} />
    </svg>
  )
}

function GridCell({ letter, status }: { letter: string; status: LetterStatus | undefined }) {
  const isCorrect = status === 'correct'
  return (
    <div
      className="relative overflow-hidden flex-1 aspect-square max-w-12 flex items-center justify-center text-sm sm:text-lg font-bold uppercase rounded border-2 transition-colors duration-300 text-white"
      style={{
        backgroundColor: isCorrect ? COLOR_CORRECT : COLOR_UNKNOWN,
        borderColor: isCorrect ? COLOR_CORRECT : COLOR_UNKNOWN,
      }}
    >
      {status === 'present' && (
        <span className="absolute inset-0 rounded-full" style={{ backgroundColor: COLOR_PRESENT }} />
      )}
      <span className={`relative ${letter === '.' ? 'self-end pb-0.5' : ''}`}>{letter === '.' ? '·' : letter}</span>
    </div>
  )
}

function scoreLabel(score: number | null): string {
  if (score === null) return 'En cours…'
  return score >= FAILED_SCORE ? "Pas pour aujourd'hui 😅" : `En ${score} coups`
}

const SHARE_EMOJI: Record<LetterStatus, string> = {
  correct: '🟥',
  present: '🟡',
  absent: '🟦',
}

function buildShareText(puzzleNumber: number, target: string, guesses: string[], score: number): string {
  const grid = guesses
    .map((g) =>
      scoreGuess(target, g)
        .map((r) => SHARE_EMOJI[r.status])
        .join(''),
    )
    .join('\n')
  return `#SUTOM #${puzzleNumber} ${scoreLabel(score)}\n\n${grid}\n\nhttps://sutom.nocle.fr via https://network.coworkinmontpellier.org`
}

// Golf-style badge for a score relative to par: eagle (2 under) beats birdie (1 under).
function golfBadge(par: number | null, score: number | null): { emoji: string; title: string } | null {
  if (par == null || score == null) return null
  if (score <= par - 2) return { emoji: '🦅', title: 'Eagle' }
  if (score === par - 1) return { emoji: '🐦', title: 'Birdie' }
  return null
}

// Mirrors the backend's leaderboard_points formula for a plain (no bonus, no catchup)
// score relative to a given par — used to render the reference table in the info modal.
function basePoints(par: number, score: number): number {
  return 3 + (par - score) / (score > par ? 2 : 1)
}

function ScoreboardPanel({ scoreboard, par }: { scoreboard: Scoreboard | null; par: number | null }) {
  const [expanded, setExpanded] = useState<number | null>(null)

  if (!scoreboard) return <span className="loading loading-spinner loading-sm text-base-content/40" />
  if (scoreboard.players.length === 0) {
    return <p className="text-sm text-base-content/40">Personne n'a encore joué aujourd'hui.</p>
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {scoreboard.players.map((p: ScoreboardPlayer, i: number) => {
        const golf = golfBadge(par, p.score)
        return (
          <li key={p.user_id} className="border border-base-200 rounded-lg overflow-hidden">
            <button
              className="w-full flex items-center justify-between px-3 py-2 text-sm disabled:cursor-default enabled:hover:bg-base-200 enabled:cursor-pointer transition-colors"
              onClick={() => setExpanded((e) => (e === p.user_id ? null : p.user_id))}
              disabled={!p.revealed}
              title={p.revealed ? 'Voir le détail' : undefined}
            >
              <span>
                {i + 1}. {p.first_name}
                {p.first_to_finish && <span title="Premier à finir"> 🥇</span>}
                {golf && <span title={golf.title}> {golf.emoji}</span>}
              </span>
              {p.finished ? (
                <span className="flex items-center gap-2">
                  <span className="badge badge-sm">{scoreLabel(p.score)}</span>
                  {p.points != null && <span className="text-xs text-base-content/60">{p.points.toFixed(2)} pts</span>}
                </span>
              ) : (
                <span className="text-xs text-base-content/40">En cours…</span>
              )}
            </button>
            {expanded === p.user_id && p.sequence && (
              <div className="px-3 py-2 flex items-start justify-between gap-3">
                <div className="flex flex-col gap-0.5">
                  {p.sequence.map((row, rowIdx) => (
                    <div key={rowIdx} className="flex gap-0.5">
                      {row.map((status, j) => (
                        <div key={j} className={`w-4 h-4 rounded-sm ${miniCellClass(status)}`} />
                      ))}
                    </div>
                  ))}
                </div>
                {par != null && p.score != null && p.points != null && (
                  <ul className="text-[11px] text-base-content/50 leading-tight pl-3">
                    <li>
                      Score {p.score} vs par {par} = {basePoints(par, p.score).toFixed(0)} pts
                    </li>
                    {p.first_to_finish && <li>Premier à finir : +0,5 pt</li>}
                    {p.is_catchup && <li>Rattrapage : × 50%</li>}
                    <li className="font-medium text-base-content/70">Total : {p.points.toFixed(2)} pts</li>
                  </ul>
                )}
              </div>
            )}
          </li>
        )
      })}
    </ul>
  )
}

function LeaderboardPanel({ leaderboard }: { leaderboard: Leaderboard | null }) {
  if (!leaderboard) return <span className="loading loading-spinner loading-sm text-base-content/40" />
  if (leaderboard.entries.length === 0) {
    return <p className="text-sm text-base-content/40">Pas encore de classement.</p>
  }

  return (
    <table className="table table-sm">
      <thead>
        <tr>
          <th></th>
          <th>Joueur</th>
          <th
            className="text-right text-base-content/40 font-normal"
            title="Indicatif uniquement, sans effet sur le classement"
          >
            Points / jour
          </th>
          <th className="text-right">Points</th>
        </tr>
      </thead>
      <tbody>
        {leaderboard.entries.map((e, i) => (
          <tr key={e.user_id}>
            <td className="text-base-content/40">{i + 1}</td>
            <td>{e.first_name}</td>
            <td className="text-right text-base-content/50">{e.points_per_day.toFixed(2)}</td>
            <td className="text-right font-semibold">{e.points.toFixed(2)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function PointsInfoModal() {
  const dialogRef = useRef<HTMLDialogElement>(null)

  return (
    <>
      <button
        className="btn btn-circle btn-ghost btn-xs"
        onClick={() => dialogRef.current?.showModal()}
        aria-label="Comment les points sont calculés"
      >
        ℹ️
      </button>
      <dialog ref={dialogRef} className="modal">
        <div className="modal-box">
          <h3 className="font-bold text-lg">Comment les points sont calculés</h3>
          <div className="py-4 text-sm flex flex-col gap-3">
            <p>
              Chaque jour, un <strong>par</strong> est calculé : la moyenne (arrondie au supérieur) des scores de tous
              les joueurs ayant terminé la grille. Tant que la journée n'est pas finie, ce par est provisoire et peut
              encore bouger ; il est ensuite figé pour toujours, même si quelqu'un complète la grille plus tard.
            </p>
            <p>
              Un score égal au par vaut donc 3 points ; chaque coup de moins en rapporte 1 point de plus, chaque coup de
              plus en retire 0,5 point.
            </p>
            <div className="overflow-x-auto">
              <table className="table table-xs text-center">
                <thead>
                  <tr>
                    <th className="text-left">Par \ Score</th>
                    {[1, 2, 3, 4, 5, 6].map((s) => (
                      <th key={s} className="text-center">
                        {s}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {[1, 2, 3, 4, 5, 6].map((par) => (
                    <tr key={par}>
                      <th className="text-left font-normal text-base-content/50">{par}</th>
                      {[1, 2, 3, 4, 5, 6].map((s) => (
                        <td key={s} className={s === par ? 'font-semibold' : ''}>
                          {basePoints(par, s).toFixed(2)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p>🥇 +0,5 point bonus pour le premier joueur à terminer la grille du jour.</p>
            <p>
              🐦 Birdie (score = par − 1) et 🦅 Eagle (score ≤ par − 2) sont affichés à côté du score, à titre
              honorifique.
            </p>
            <p>
              Rejouer une grille des jours précédents ne rapporte que <strong>50 %</strong> des points obtenus (bonus du
              premier compris).
            </p>
          </div>
          <div className="modal-action">
            <form method="dialog">
              <button className="btn">Fermer</button>
            </form>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </>
  )
}

function parseDateKey(dateStr: string): Date {
  const [y, m, d] = dateStr.split('-').map(Number)
  return new Date(y, m - 1, d)
}

function formatDateKey(date: Date): string {
  return format(date, 'yyyy-MM-dd')
}

export function Sutom() {
  // `undefined` means "today". Kept as plain state rather than a URL param, so
  // browsing history stays a single /sutom URL with buttons driving the date instead.
  const [viewedDate, setViewedDate] = useState<string | undefined>(undefined)
  const [day, setDay] = useState<Day | null>(null)
  const [guesses, setGuesses] = useState<string[]>([])
  const [score, setScore] = useState<number | null>(null)
  const [currentGuess, setCurrentGuess] = useState('')
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState(false)
  // Bumped by the retry button to re-run the day load after a failure.
  const [loadAttempt, setLoadAttempt] = useState(0)
  const [submitting, setSubmitting] = useState(false)
  // How many letters of the most recently submitted row currently show their color.
  // Starts at Infinity so guesses loaded from the server render fully revealed.
  const [revealCount, setRevealCount] = useState(Infinity)
  const [scoreboard, setScoreboard] = useState<Scoreboard | null>(null)
  const [leaderboard, setLeaderboard] = useState<Leaderboard | null>(null)
  const [includeToday, setIncludeToday] = useState(true)
  const [history, setHistory] = useState<History | null>(null)
  const notify = useToast()

  useEffect(() => {
    // Reset everything up front: switching days no longer remounts the component,
    // so stale state from the previous day must be cleared explicitly.
    setLoading(true)
    setLoadError(false)
    setDay(null)
    setGuesses([])
    setScore(null)
    setCurrentGuess('')
    setRevealCount(Infinity)
    setScoreboard(null)
    ;(viewedDate ? fetchDay(viewedDate) : fetchToday())
      .then((d) => {
        setDay(d)
        setGuesses(d.my_guesses)
        setScore(d.my_score)
        setCurrentGuess(d.word[0] ?? '')
      })
      .catch(() => setLoadError(true))
      .finally(() => setLoading(false))

    fetchHistory()
      .then(setHistory)
      .catch(() => {
        /* non-fatal */
      })
  }, [viewedDate, loadAttempt])

  const refreshLeaderboard = useCallback((include: boolean) => {
    fetchLeaderboard(include)
      .then(setLeaderboard)
      .catch(() => {
        /* non-fatal */
      })
  }, [])

  // Independent of which day is being viewed — re-fetched only when the toggle changes.
  useEffect(() => {
    refreshLeaderboard(includeToday)
  }, [includeToday, refreshLeaderboard])

  const refreshScoreboard = useCallback((date: string) => {
    fetchScoreboard(date)
      .then(setScoreboard)
      .catch(() => {
        /* non-fatal */
      })
  }, [])

  useEffect(() => {
    if (day) refreshScoreboard(day.date)
  }, [day, refreshScoreboard])

  const gameOver = score !== null
  const solved = score !== null && score <= MAX_ATTEMPTS

  const submitGuess = useCallback(() => {
    if (!day || gameOver || submitting) return
    if (currentGuess.length !== day.word.length) {
      notify('Le mot est trop court.', 'error')
      return
    }
    if (currentGuess.includes('.')) {
      notify('Remplacez les points par de vraies lettres avant de valider.', 'error')
      return
    }
    if (!day.possible_words.includes(currentGuess)) {
      notify("Ce mot n'est pas dans notre dictionnaire.", 'error')
      return
    }

    const submittedGuess = currentGuess
    const nextGuesses = [...guesses, submittedGuess]
    const isWin = submittedGuess === day.word
    setSubmitting(true)
    submitGuesses(day.date, nextGuesses)
      .then(async (result) => {
        setGuesses(nextGuesses)
        setCurrentGuess(day.word[0] ?? '')

        // Reveal the row's letters one at a time before acting on the outcome, so
        // the color feedback stays suspenseful instead of popping in all at once.
        setRevealCount(0)
        for (let revealed = 1; revealed <= day.word.length; revealed++) {
          await sleep(REVEAL_DELAY_MS)
          setRevealCount(revealed)
        }

        setScore(result.score)
        if (isWin) {
          notify('Bravo, mot trouvé !', 'success')
        } else if (result.score !== null) {
          notify(`Perdu ! Le mot était ${day.word}.`, 'info')
        }
        if (result.score !== null) {
          refreshScoreboard(day.date)
          refreshLeaderboard(includeToday)
        }
      })
      .catch((e) => notify(e instanceof ApiError ? e.message : 'Erreur, réessayez.', 'error'))
      .finally(() => setSubmitting(false))
  }, [day, gameOver, submitting, currentGuess, guesses, notify, refreshScoreboard, refreshLeaderboard, includeToday])

  const typeLetter = useCallback(
    (letter: string) => {
      if (!day || gameOver || submitting) return
      setCurrentGuess((g) => (g.length < day.word.length ? g + letter : g))
    },
    [day, gameOver, submitting],
  )

  const backspace = useCallback(() => {
    setCurrentGuess((g) => (g.length > 1 ? g.slice(0, -1) : g))
  }, [])

  const copySummary = useCallback(() => {
    if (!day || score === null) return
    const text = buildShareText(day.puzzle_number, day.word, guesses, score)
    navigator.clipboard
      .writeText(text)
      .then(() => notify('Résumé copié !', 'success'))
      .catch(() => notify('Impossible de copier le résumé.', 'error'))
  }, [day, score, guesses, notify])

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!day || gameOver || submitting) return
      if (e.key === 'Enter') {
        submitGuess()
        return
      }
      if (e.key === 'Backspace') {
        backspace()
        return
      }
      const letter = e.key.toUpperCase()
      if (/^[A-Z.]$/.test(letter)) typeLetter(letter)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [day, gameOver, submitting, submitGuess, backspace, typeLetter])

  if (loading) {
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <Navbar />
        <main className="flex-1 flex items-center justify-center">
          <span className="loading loading-spinner loading-lg text-base-content/40" />
        </main>
      </div>
    )
  }

  if (loadError || !day) {
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <Navbar />
        <main className="flex-1 flex flex-col items-center justify-center gap-3">
          <p className="text-sm text-base-content/60">Impossible de charger ce mot du jour.</p>
          <button className="btn btn-sm btn-primary" onClick={() => setLoadAttempt((n) => n + 1)}>
            Réessayer
          </button>
        </main>
      </div>
    )
  }

  const wordLength = day.word.length
  const keyStatuses = keyboardStatuses(day.word, guesses)

  const currentUserId = getTokenPayload()?.sub
  const isFirstToFinish = scoreboard?.players.some((p) => p.user_id === currentUserId && p.first_to_finish) ?? false
  const winMessage =
    score !== null && score <= MAX_ATTEMPTS
      ? `🎉 Bravo, vous avez trouvé le mot en ${score} coup${score === 1 ? '' : 's'} ! 🎉`
      : `Le mot était ${day.word}.`

  const todayDate = history?.entries[0]?.date
  // Any day in the last HISTORY_WINDOW_DAYS is playable (lazily fetched server-side),
  // not just the ones already cached in `history`.
  const oldestPlayableDate = todayDate
    ? formatDateKey(addDays(parseDateKey(todayDate), -(HISTORY_WINDOW_DAYS - 1)))
    : undefined
  const canGoPrev = !!oldestPlayableDate && day.date > oldestPlayableDate
  const canGoNext = !!todayDate && day.date < todayDate
  const goToDate = (date: string) => setViewedDate(date === todayDate ? undefined : date)

  // `history.entries` is sorted most-recent-first, so the first unfinished entry is the
  // most recent one — handy for jumping straight to whatever's still left to play.
  const lastUnfinishedDate = history?.entries.find((h) => h.my_score == null)?.date
  const canGoToLastUnfinished = !!lastUnfinishedDate && lastUnfinishedDate !== day.date

  // While the just-submitted row is still revealing letter-by-letter, the "next guess"
  // preview row below it (dots, or the win/loss outcome) must stay hidden — otherwise it
  // pops in immediately, ahead of the animation it's supposed to follow.
  const isRevealingLastRow = guesses.length > 0 && revealCount < wordLength

  const rows: Array<{
    letters: string[]
    statuses: (LetterStatus | undefined)[]
  }> = []
  for (let i = 0; i < MAX_ATTEMPTS; i++) {
    if (i < guesses.length) {
      const result = scoreGuess(day.word, guesses[i])
      const isRevealing = i === guesses.length - 1 && revealCount < wordLength
      rows.push({
        // Letters stay visible right away (you typed them, so there's nothing to hide) —
        // only the color reveal is staged, one tile at a time.
        letters: result.map((r) => r.letter),
        statuses: result.map((r, j) => (isRevealing && j >= revealCount ? undefined : r.status)),
      })
    } else if (i === guesses.length && !gameOver && !isRevealingLastRow) {
      // The line you're about to fill in: first letter shown crisp, untyped slots
      // marked with a placeholder dot so the word's length stays visible.
      const letters = currentGuess.split('')
      while (letters.length < wordLength) letters.push('.')
      rows.push({ letters, statuses: Array(wordLength).fill(undefined) })
    } else {
      // Not-yet-reached lines stay blank — only the very next line previews the first letter.
      rows.push({
        letters: Array(wordLength).fill(''),
        statuses: Array(wordLength).fill(undefined),
      })
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="flex-1 w-full p-4 md:p-8 sm:max-w-xl sm:mx-auto flex flex-col gap-6">
        <div className="flex flex-col items-center gap-6 w-full">
          <div className="flex items-center justify-between gap-1 sm:gap-3 w-full">
            <div className="flex items-center gap-1">
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => lastUnfinishedDate && goToDate(lastUnfinishedDate)}
                disabled={!canGoToLastUnfinished}
                aria-label="Dernière grille non terminée"
                title="Dernière grille non terminée"
              >
                <ChevronIcon path={CHEVRON_DOUBLE_LEFT} />
              </button>
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => goToDate(formatDateKey(addDays(parseDateKey(day.date), -1)))}
                disabled={!canGoPrev}
                aria-label="Jour précédent"
                title="Jour précédent"
              >
                <ChevronIcon path={CHEVRON_LEFT} />
              </button>
            </div>
            <div className="flex-1 text-center">
              <h2 className="text-xl font-bold">SUTOM #{day.puzzle_number}</h2>
              <div className="text-sm text-base-content/60">
                {format(parseDateKey(day.date), 'd MMMM yyyy', { locale: fr })}
              </div>
            </div>
            <div className="flex items-center gap-1">
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => goToDate(formatDateKey(addDays(parseDateKey(day.date), 1)))}
                disabled={!canGoNext}
                aria-label="Jour suivant"
                title="Jour suivant"
              >
                <ChevronIcon path={CHEVRON_RIGHT} />
              </button>
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => todayDate && goToDate(todayDate)}
                disabled={!canGoNext}
                aria-label="Aujourd'hui"
                title="Aujourd'hui"
              >
                <ChevronIcon path={CHEVRON_DOUBLE_RIGHT} />
              </button>
            </div>
          </div>

          {day.date < LEADERBOARD_START_DATE && (
            <div role="alert" className="alert alert-warning w-full text-sm">
              <span>
                Partie d'entraînement : le concours du classement général a démarré mercredi 23 septembre 2026, toutes
                les parties antérieures à cette date ne rapportent pas de point au classement.
              </span>
            </div>
          )}

          {gameOver && (
            <div
              className={`alert ${solved ? 'alert-success' : 'alert-info'} w-full flex flex-col gap-2 sm:gap-3 items-center justify-center`}
            >
              <span className="text-center">{winMessage}</span>
              {solved && isFirstToFinish && (
                <span className="text-center font-semibold">🥇 Et vous êtes le premier ce jour ! 🥇</span>
              )}
              <button className="btn btn-sm btn-outline" onClick={copySummary}>
                Copier le résumé
              </button>
            </div>
          )}

          <div className="flex flex-col gap-1 sm:gap-1.5 w-full">
            {rows.map((row, i) => (
              <div key={i} className="flex gap-1 sm:gap-1.5 justify-center">
                {row.letters.map((letter, j) => (
                  <GridCell key={j} letter={letter} status={row.statuses[j]} />
                ))}
              </div>
            ))}
          </div>

          <div className="flex flex-col gap-1.5 sm:gap-2 w-full">
            {KEYBOARD_ROWS.map((row, i) => (
              <div key={i} className="flex gap-1">
                {row.map((key) => {
                  if (key === 'ENTER') {
                    return (
                      <button
                        key={key}
                        className="btn btn-sm sm:btn-md btn-primary px-0 flex-[2]"
                        onClick={submitGuess}
                        disabled={gameOver || submitting}
                      >
                        Entrée
                      </button>
                    )
                  }
                  if (key === 'BACKSPACE') {
                    return (
                      <button
                        key={key}
                        className="btn btn-sm sm:btn-md btn-outline px-0 flex-1"
                        onClick={backspace}
                        disabled={gameOver || submitting}
                      >
                        ⌫
                      </button>
                    )
                  }
                  return (
                    <button
                      key={key}
                      className={`btn btn-sm sm:btn-md px-0 flex-1 ${keyStyle(keyStatuses[key]) ? '' : 'btn-outline'}`}
                      style={keyStyle(keyStatuses[key])}
                      onClick={() => typeLetter(key)}
                      disabled={gameOver || submitting}
                    >
                      {key === '.' ? <span className="self-end pb-0.5">·</span> : key}
                    </button>
                  )
                })}
              </div>
            ))}
          </div>
        </div>

        <hr className="border-base-300" />

        <div className="flex flex-col gap-6">
          <div className="card bg-base-100 shadow-sm">
            <div className="card-body p-4">
              <h3 className="font-semibold text-sm mb-2">
                Classement du jour
                {day.par != null && (
                  <span
                    className="text-base-content/40 font-normal"
                    title={day.par_average != null ? `Moyenne actuelle : ${day.par_average.toFixed(2)}` : undefined}
                  >
                    {' '}
                    · par {day.par}
                    {day.date === todayDate ? ' (provisoire)' : ''}
                  </span>
                )}
              </h3>
              <ScoreboardPanel scoreboard={scoreboard} par={day.par} />
            </div>
          </div>
          <div className="card bg-base-100 shadow-sm">
            <div className="card-body p-4">
              <div className="flex items-center justify-between mb-2 gap-2">
                <h3 className="font-semibold text-sm">
                  Classement — 30 derniers jours{' '}
                  <span className="font-normal text-xs text-base-content/50 ml-2">
                    <input
                      type="checkbox"
                      className="toggle toggle-xs align-middle mx-1"
                      checked={includeToday}
                      onChange={(e) => setIncludeToday(e.target.checked)}
                      title="Inclure les résultats provisoires du jour dans le classement"
                    />
                    dont aujourd'hui
                  </span>
                </h3>
                <PointsInfoModal />
              </div>
              <LeaderboardPanel leaderboard={leaderboard} />
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
