import { addDays, addMonths, format } from 'date-fns'
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
// Earliest month the monthly leaderboard can be browsed back to.
const FIRST_LEADERBOARD_MONTH = '2026-10'

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
  ['.', 'W', 'X', 'C', 'V', 'B', 'N', 'BACKSPACE', 'ENTER'],
]

// The original SUTOM's own palette: unfilled/not-found letters stay the grid's blue,
// well-placed letters turn the whole tile red, and misplaced letters get a yellow
// circle rather than a full-tile color (source: sutom.nocle.fr's jeu.css).
const COLOR_UNKNOWN = '#0077c7'
const COLOR_CORRECT = '#e7002a'
const COLOR_PRESENT = '#ffbd00'
const COLOR_ABSENT_KEY = '#707070'

// An untyped slot of the line being typed. Rendered as a faint dot so it can't be confused
// with a "." the player typed on purpose as a placeholder while thinking.
const EMPTY_SLOT = '_'

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
      // Dimmed so a tried-and-absent key reads as distinct from a still-untried one.
      return { borderColor: COLOR_ABSENT_KEY, color: COLOR_ABSENT_KEY, opacity: 0.5 }
    default:
      return undefined
  }
}

// Icons as SVG paths rather than text glyphs or emoji, so they sit dead-center in a
// circular button regardless of font metrics.
const CHEVRON_LEFT = 'M15.75 19.5 8.25 12l7.5-7.5'
const CHEVRON_RIGHT = 'M8.25 4.5l7.5 7.5-7.5 7.5'
const ICON_CALENDAR =
  'M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 7.5v11.25m-18 0A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75m-18 0v-7.5A2.25 2.25 0 0 1 5.25 9h13.5A2.25 2.25 0 0 1 21 11.25v7.5'
const ICON_HOURGLASS = 'M6 3h12M6 21h12M7.5 3c0 4.5 4.5 6 4.5 9s-4.5 4.5-4.5 9M16.5 3c0 4.5-4.5 6-4.5 9s4.5 4.5 4.5 9'

function OutlineIcon({ path, size = 'w-4 h-4' }: { path: string; size?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className={size}>
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
      {letter === '.' || letter === EMPTY_SLOT ? (
        <span className={`relative self-end pb-0.5 ${letter === EMPTY_SLOT ? 'opacity-40' : ''}`}>·</span>
      ) : (
        <span className="relative">{letter}</span>
      )}
    </div>
  )
}

// Appends the last name's initial to disambiguate players sharing a first name.
function displayName(firstName: string, lastInitial: string): string {
  return lastInitial ? `${firstName} ${lastInitial}.` : firstName
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

// Monthly wins as stars, every five of them collapsed into a shooting star:
// 7 wins → 💫⭐⭐.
function winStars(wins: number): string {
  return '💫'.repeat(Math.floor(wins / 5)) + '⭐'.repeat(wins % 5)
}

// Badge for a game finished after its own day: next day, or later.
function lateBadge(daysLate: number): { emoji: string; title: string } | null {
  if (daysLate <= 0) return null
  if (daysLate === 1) return { emoji: '🌓', title: 'Terminé le lendemain (× 75 %)' }
  return { emoji: '⏰', title: `Terminé avec ${daysLate} jours de retard (× 50 %)` }
}

// Mirrors the backend's leaderboard_points formula for a plain (no bonus, on time)
// score relative to a given par — used to render the reference table in the info modal.
function basePoints(par: number, score: number): number {
  return 3 + (par - score) / (score > par ? 2 : 1)
}

function ScoreboardPanel({ scoreboard, par }: { scoreboard: Scoreboard | null; par: number | null }) {
  const [expanded, setExpanded] = useState<Set<number>>(new Set())

  if (!scoreboard) return <span className="loading loading-spinner loading-sm text-base-content/40" />
  if (scoreboard.players.length === 0) {
    return <p className="text-sm text-base-content/40">Personne n'a encore joué aujourd'hui.</p>
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {scoreboard.players.map((p: ScoreboardPlayer, i: number) => {
        const golf = golfBadge(par, p.score)
        const late = lateBadge(p.days_late)
        return (
          <li key={p.user_id} className="border border-base-200 rounded-lg overflow-hidden">
            <button
              className="w-full flex items-center justify-between px-3 py-2 text-sm disabled:cursor-default enabled:hover:bg-base-200 enabled:cursor-pointer transition-colors"
              onClick={() =>
                setExpanded((prev) => {
                  const next = new Set(prev)
                  if (next.has(p.user_id)) next.delete(p.user_id)
                  else next.add(p.user_id)
                  return next
                })
              }
              disabled={!p.revealed}
              title={p.revealed ? 'Voir le détail' : undefined}
            >
              <span>
                {i + 1}. {displayName(p.first_name, p.last_initial)}
                {p.first_to_finish && <span title="Premier à finir"> 🥇</span>}
                {p.fast && <span title="Trouvé en moins d'une minute"> ⚡</span>}
                {late && <span title={late.title}> {late.emoji}</span>}
                {p.score === 1 ? (
                  <span title="Headshot"> 🎯</span>
                ) : (
                  golf && <span title={golf.title}> {golf.emoji}</span>
                )}
              </span>
              {p.finished ? (
                <span className="flex items-center gap-2">
                  <span className="badge badge-sm">{scoreLabel(p.score)}</span>
                  {p.points != null && (
                    <span className="text-xs text-base-content/60">{p.points.toFixed(2)} points</span>
                  )}
                </span>
              ) : (
                <span className="text-xs text-base-content/40">En cours…</span>
              )}
            </button>
            {expanded.has(p.user_id) && p.sequence && (
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
                      Score {p.score} vs par {par} = {basePoints(par, p.score).toFixed(1)} points
                    </li>
                    {p.first_to_finish && <li>Premier à finir : +0,5 point</li>}
                    {p.fast && <li>Moins d'une minute : +0,5 point</li>}
                    {p.days_late === 1 && <li>Le lendemain : × 75 %</li>}
                    {p.days_late > 1 && <li>En retard : × 50 %</li>}
                    <li className="font-medium text-base-content/70">Total : {p.points.toFixed(2)} points</li>
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
            title="Départage les égalités de score (le plus bas l'emporte) — les jours non joués ne comptent pas"
          >
            Coups moyens
          </th>
          <th className="text-right">Score</th>
        </tr>
      </thead>
      <tbody>
        {leaderboard.entries.map((e, i) => (
          <tr key={e.user_id}>
            <td className="text-base-content/40">{i + 1}</td>
            <td>
              {displayName(e.first_name, e.last_initial)}
              {e.won_months.length > 0 && (
                <span title={`Vainqueur : ${e.won_months.map(monthLabel).join(', ')}`}>
                  {' '}
                  {winStars(e.won_months.length)}
                </span>
              )}
            </td>
            <td className="text-right text-base-content/50">{e.avg_guesses.toFixed(2)}</td>
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
          <form method="dialog">
            <button className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" aria-label="Fermer">
              ✕
            </button>
          </form>
          <h3 className="font-bold text-lg pr-8">Comment les points sont calculés ?</h3>
          <div className="py-4 text-sm flex flex-col gap-3">
            <p>
              Chaque jour, un <strong>par</strong> est calculé : la moyenne (arrondie au supérieur) des scores de tous
              les joueurs ayant terminé la grille.
            </p>
            <p>
              Tant que la journée n'est pas finie, ce <strong>par</strong> est provisoire et peut encore bouger ; il est
              ensuite figé pour toujours, même si quelqu'un complète la grille plus tard.
            </p>
            <p>
              Un score égal au <strong>par</strong> vaut donc 3 points ; chaque coup de moins en rapporte 1 point de
              plus, chaque coup de plus en retire 0,5 point.
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
            <p>Bonus :</p>
            <ul className="list-disc pl-5 -mt-2">
              <li>
                🥇 +0,5 point pour le <strong>premier</strong> joueur à terminer la grille du jour ;
              </li>
              <li>
                ⚡ +0,5 point si le mot est trouvé moins d'<strong>une minute</strong> après la première proposition.
              </li>
            </ul>
            <p>Malus de retard :</p>
            <ul className="list-disc pl-5 -mt-2">
              <li>
                🌓 grille terminée le lendemain : <strong>75 %</strong> des points ;
              </li>
              <li>
                ⏰ terminée plus tard : <strong>50 %</strong> des points.
              </li>
            </ul>
            <p>Badges honorifiques :</p>
            <ul className="list-disc pl-5 -mt-2">
              <li>🎯 Headshot : trouvé du premier coup</li>
              <li>
                🦅 Eagle : trouvé en <strong>par</strong> − 2 tentatives
              </li>
              <li>
                🐦 Birdie : trouvé en <strong>par</strong> − 1 tentatives
              </li>
            </ul>
            <p>Le classement est mensuel, il est réinitialisé le 1er de chaque mois. </p>
            <p>Celui qui termine le premier du mois obtient une ⭐ à côté de son nom et 💫 toutes les 5 étoiles.</p>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </>
  )
}

function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

// "2026-10" → "octobre 2026".
function monthLabel(month: string): string {
  return format(parseDateKey(`${month}-01`), 'MMMM yyyy', { locale: fr })
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

  const refreshLeaderboard = useCallback((month: string) => {
    fetchLeaderboard(month)
      .then(setLeaderboard)
      .catch(() => {
        /* non-fatal */
      })
  }, [])

  // The leaderboard follows the month of the day being viewed, but can also be browsed
  // month by month on its own; changing day into another month snaps it back.
  const viewedMonth = day?.date.slice(0, 7)
  const [leaderboardMonth, setLeaderboardMonth] = useState<string | undefined>(undefined)
  useEffect(() => {
    if (viewedMonth) setLeaderboardMonth(viewedMonth)
  }, [viewedMonth])
  useEffect(() => {
    if (!leaderboardMonth) return
    setLeaderboard(null)
    refreshLeaderboard(leaderboardMonth)
  }, [leaderboardMonth, refreshLeaderboard])

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
          if (leaderboardMonth) refreshLeaderboard(leaderboardMonth)
        }
      })
      .catch((e) => notify(e instanceof ApiError ? e.message : 'Erreur, réessayez.', 'error'))
      .finally(() => setSubmitting(false))
  }, [
    day,
    gameOver,
    submitting,
    currentGuess,
    guesses,
    notify,
    refreshScoreboard,
    refreshLeaderboard,
    leaderboardMonth,
  ])

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
  // While the just-submitted row is still revealing letter-by-letter, the keyboard must
  // not jump ahead of it — exclude that guess from the keyboard's colors until the
  // animation finishes, so both update in sync instead of the keyboard spoiling it early.
  const isRevealingLastRow = guesses.length > 0 && revealCount < wordLength
  const keyStatuses = keyboardStatuses(day.word, isRevealingLastRow ? guesses.slice(0, -1) : guesses)

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

  // Jumps to a grid still left to play: the most recent one, or, when the day shown is
  // itself still pending, the next older pending one. `history.entries` is sorted
  // most-recent-first; the current day's status comes from `score`, which is up to date
  // even right after finishing it.
  const pendingDays = history?.entries.filter((h) => h.my_score == null && h.date !== day.date) ?? []
  const lastUnfinishedDate = (score === null ? pendingDays.find((h) => h.date < day.date) : pendingDays[0])?.date

  const shownMonth = leaderboardMonth ?? day.date.slice(0, 7)
  const currentMonth = (todayDate ?? day.date).slice(0, 7)
  const shiftMonth = (delta: number) =>
    setLeaderboardMonth(format(addMonths(parseDateKey(`${shownMonth}-01`), delta), 'yyyy-MM'))
  const canGoToLastUnfinished = !!lastUnfinishedDate

  // Positions confirmed correct by any earlier guess this game, e.g. to remind the
  // player of them on a fresh, untouched line — cleared the moment they start typing.
  const knownCorrectLetters: (string | null)[] = Array(wordLength).fill(null)
  guesses.forEach((g) => {
    scoreGuess(day.word, g).forEach((r, pos) => {
      if (r.status === 'correct') knownCorrectLetters[pos] = r.letter
    })
  })

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
      while (letters.length < wordLength) letters.push(EMPTY_SLOT)
      const statuses: (LetterStatus | undefined)[] = Array(wordLength).fill(undefined)
      // Nothing typed yet beyond the given first letter: remind the player of
      // already-confirmed correct-position letters. The instant they type, this
      // reminder disappears and typing proceeds normally from a blank line.
      if (currentGuess.length <= 1) {
        knownCorrectLetters.forEach((known, pos) => {
          if (known) {
            letters[pos] = known
            statuses[pos] = 'correct'
          }
        })
      }
      rows.push({ letters, statuses })
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
                aria-label="Dernière grille en attente"
                title="Dernière grille en attente"
              >
                <OutlineIcon path={ICON_HOURGLASS} />
              </button>
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => goToDate(formatDateKey(addDays(parseDateKey(day.date), -1)))}
                disabled={!canGoPrev}
                aria-label="Jour précédent"
                title="Jour précédent"
              >
                <OutlineIcon path={CHEVRON_LEFT} />
              </button>
            </div>
            <div className="flex-1 text-center">
              <h2 className="text-xl font-bold">SUTOM #{day.puzzle_number}</h2>
              <div className="text-sm text-base-content/60">
                {capitalize(format(parseDateKey(day.date), 'EEEE d MMMM yyyy', { locale: fr }))}
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
                <OutlineIcon path={CHEVRON_RIGHT} />
              </button>
              <button
                className="btn btn-circle btn-outline btn-primary"
                onClick={() => todayDate && goToDate(todayDate)}
                disabled={!canGoNext}
                aria-label="Aujourd'hui"
                title="Aujourd'hui"
              >
                <OutlineIcon path={ICON_CALENDAR} />
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
                <h3 className="font-semibold text-sm flex items-center gap-1">
                  Classement —
                  <button
                    className="btn btn-circle btn-ghost btn-xs"
                    onClick={() => shiftMonth(-1)}
                    disabled={shownMonth <= FIRST_LEADERBOARD_MONTH}
                    aria-label="Mois précédent"
                    title="Mois précédent"
                  >
                    <OutlineIcon path={CHEVRON_LEFT} size="w-3 h-3" />
                  </button>
                  {monthLabel(shownMonth)}
                  <button
                    className="btn btn-circle btn-ghost btn-xs"
                    onClick={() => shiftMonth(1)}
                    disabled={shownMonth >= currentMonth}
                    aria-label="Mois suivant"
                    title="Mois suivant"
                  >
                    <OutlineIcon path={CHEVRON_RIGHT} size="w-3 h-3" />
                  </button>
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
