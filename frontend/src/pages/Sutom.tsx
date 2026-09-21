import { useCallback, useEffect, useState } from 'react'
import { fetchSutomToday, type Today } from '../api/sutom'
import { Navbar } from '../components/Navbar'
import { useToast } from '../lib/toast'

const MAX_ATTEMPTS = 6

type LetterStatus = 'correct' | 'present' | 'absent'
interface LetterResult { letter: string; status: LetterStatus }

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

const STATUS_PRIORITY: Record<LetterStatus, number> = { absent: 0, present: 1, correct: 2 }

function keyboardStatuses(target: string, guesses: string[]): Record<string, LetterStatus> {
  const statuses: Record<string, LetterStatus> = {}
  guesses.forEach(guess => {
    scoreGuess(target, guess).forEach(({ letter, status }) => {
      const current = statuses[letter]
      if (!current || STATUS_PRIORITY[status] > STATUS_PRIORITY[current]) statuses[letter] = status
    })
  })
  return statuses
}

function storageKey(date: string): string {
  return `sutom:${date}`
}

function loadGuesses(date: string): string[] {
  try {
    const raw = localStorage.getItem(storageKey(date))
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed.filter((g): g is string => typeof g === 'string') : []
  } catch {
    return []
  }
}

function saveGuesses(date: string, guesses: string[]) {
  try {
    localStorage.setItem(storageKey(date), JSON.stringify(guesses))
  } catch { /* storage unavailable, progress just won't persist */ }
}

const KEYBOARD_ROWS = [
  ['A', 'Z', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'],
  ['Q', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'M'],
  ['ENTER', 'W', 'X', 'C', 'V', 'B', 'N', 'BACKSPACE'],
]

function cellClass(status: LetterStatus | undefined): string {
  switch (status) {
    case 'correct': return 'bg-success text-success-content border-success'
    case 'present': return 'bg-warning text-warning-content border-warning'
    case 'absent': return 'bg-neutral text-neutral-content border-neutral'
    default: return 'bg-base-100 border-base-300'
  }
}

function keyClass(status: LetterStatus | undefined): string {
  switch (status) {
    case 'correct': return 'btn-success'
    case 'present': return 'btn-warning'
    case 'absent': return 'btn-neutral'
    default: return 'btn-outline'
  }
}

export function Sutom() {
  const [today, setToday] = useState<Today | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState(false)
  const [guesses, setGuesses] = useState<string[]>([])
  const [currentGuess, setCurrentGuess] = useState('')
  const notify = useToast()

  useEffect(() => {
    fetchSutomToday()
      .then(t => {
        setToday(t)
        setGuesses(loadGuesses(t.date))
        setCurrentGuess(t.word[0] ?? '')
      })
      .catch(() => setLoadError(true))
      .finally(() => setLoading(false))
  }, [])

  const solved = today !== null && guesses.includes(today.word)
  const gameOver = solved || guesses.length >= MAX_ATTEMPTS

  const submitGuess = useCallback(() => {
    if (!today || gameOver) return
    if (currentGuess.length !== today.word.length) {
      notify('Le mot est trop court.', 'error')
      return
    }
    if (!today.possible_words.includes(currentGuess)) {
      notify("Ce mot n'est pas dans notre dictionnaire.", 'error')
      return
    }

    const nextGuesses = [...guesses, currentGuess]
    setGuesses(nextGuesses)
    saveGuesses(today.date, nextGuesses)
    setCurrentGuess(today.word[0] ?? '')

    if (currentGuess === today.word) {
      notify('Bravo, mot trouvé !', 'success')
    } else if (nextGuesses.length >= MAX_ATTEMPTS) {
      notify(`Perdu ! Le mot était ${today.word}.`, 'info')
    }
  }, [today, gameOver, currentGuess, guesses, notify])

  const typeLetter = useCallback((letter: string) => {
    if (!today || gameOver) return
    setCurrentGuess(g => (g.length < today.word.length ? g + letter : g))
  }, [today, gameOver])

  const backspace = useCallback(() => {
    setCurrentGuess(g => (g.length > 1 ? g.slice(0, -1) : g))
  }, [])

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!today || gameOver) return
      if (e.key === 'Enter') { submitGuess(); return }
      if (e.key === 'Backspace') { backspace(); return }
      const letter = e.key.toUpperCase()
      if (/^[A-Z]$/.test(letter)) typeLetter(letter)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [today, gameOver, submitGuess, backspace, typeLetter])

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

  if (loadError || !today) {
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <Navbar />
        <main className="flex-1 flex items-center justify-center">
          <p className="text-sm text-base-content/60">Impossible de charger le mot du jour.</p>
        </main>
      </div>
    )
  }

  const firstLetter = today.word[0]
  const wordLength = today.word.length
  const keyStatuses = keyboardStatuses(today.word, guesses)

  const rows: Array<{ letters: string[]; statuses: (LetterStatus | undefined)[] }> = []
  for (let i = 0; i < MAX_ATTEMPTS; i++) {
    if (i < guesses.length) {
      const result = scoreGuess(today.word, guesses[i])
      rows.push({ letters: result.map(r => r.letter), statuses: result.map(r => r.status) })
    } else if (i === guesses.length && !gameOver) {
      const letters = currentGuess.split('')
      while (letters.length < wordLength) letters.push('')
      rows.push({ letters, statuses: letters.map(() => undefined) })
    } else {
      rows.push({
        letters: Array.from({ length: wordLength }, (_, j) => (j === 0 ? firstLetter : '')),
        statuses: Array(wordLength).fill(undefined),
      })
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="flex-1 p-4 md:p-8 max-w-xl mx-auto w-full flex flex-col items-center gap-6">
        <h2 className="text-xl font-bold">SUTOM du jour</h2>

        {gameOver && (
          <div className={`alert ${solved ? 'alert-success' : 'alert-info'} max-w-md`}>
            <span>{solved ? 'Bravo, vous avez trouvé le mot !' : `Le mot était ${today.word}.`}</span>
          </div>
        )}

        <div className="flex flex-col gap-1.5">
          {rows.map((row, i) => (
            <div key={i} className="flex gap-1.5">
              {row.letters.map((letter, j) => (
                <div
                  key={j}
                  className={`w-11 h-11 md:w-12 md:h-12 flex items-center justify-center text-lg font-bold uppercase rounded border-2 ${cellClass(row.statuses[j])} ${j === 0 && !row.statuses[j] ? 'text-base-content/50' : ''}`}
                >
                  {letter}
                </div>
              ))}
            </div>
          ))}
        </div>

        <div className="flex flex-col gap-1.5 w-full max-w-md">
          {KEYBOARD_ROWS.map((row, i) => (
            <div key={i} className="flex gap-1 justify-center">
              {row.map(key => {
                if (key === 'ENTER') {
                  return (
                    <button key={key} className="btn btn-sm btn-primary px-3" onClick={submitGuess} disabled={gameOver}>
                      Entrée
                    </button>
                  )
                }
                if (key === 'BACKSPACE') {
                  return (
                    <button key={key} className="btn btn-sm btn-outline px-3" onClick={backspace} disabled={gameOver}>
                      ⌫
                    </button>
                  )
                }
                return (
                  <button
                    key={key}
                    className={`btn btn-sm w-8 px-0 ${keyClass(keyStatuses[key])}`}
                    onClick={() => typeLetter(key)}
                    disabled={gameOver}
                  >
                    {key}
                  </button>
                )
              })}
            </div>
          ))}
        </div>
      </main>
    </div>
  )
}
