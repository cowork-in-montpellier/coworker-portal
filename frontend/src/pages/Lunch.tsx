import { format, parseISO } from 'date-fns'
import { fr } from 'date-fns/locale'
import * as A from 'fp-ts/Array'
import * as N from 'fp-ts/number'
import * as O from 'fp-ts/Ord'
import { pipe } from 'fp-ts/function'
import { useCallback, useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react'
import {
  createPlace,
  deletePlace,
  fetchToday,
  PlaceInputSchema,
  setIntention,
  setRating,
  updatePlace,
  WEEKDAY_LABELS,
  WEEKDAYS,
  type Goer,
  type Place,
  type PlaceInput,
  type Today,
  type Weekday,
} from '../api/lunch'
import { getTokenPayload } from '../auth'
import { Navbar } from '../components/Navbar'
import { PageHeader } from '../components/PageHeader'
import { CUISINE_ICONS, cuisineIcon } from '../lib/cuisineIcons'
import { useToast } from '../lib/toast'
import '../styles/lunch.css'

// How often today's choices are fetched again while the page is open, to see others' choices.
const REFRESH_MS = 60_000

type SortKey = 'rating' | 'goers' | 'name'

const SORT_LABELS: Record<SortKey, string> = {
  rating: 'Les mieux notés',
  goers: 'Les plus courus ce midi',
  name: 'Par nom',
}

// ── Icons (Font Awesome Free 6, solid) ───────────────────────────────────────

function Icon({ viewBox, d, className = '' }: { viewBox: string; d: string; className?: string }) {
  return (
    <svg viewBox={viewBox} className={`inline-block h-[1em] w-auto fill-current ${className}`} aria-hidden="true">
      <path d={d} />
    </svg>
  )
}

const icons = {
  location: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 384 512"
      d="M215.7 499.2C267 435 384 279.4 384 192C384 86 298 0 192 0S0 86 0 192c0 87.4 117 243 168.3 307.2c12.3 15.3 35.1 15.3 47.4 0zM192 128a64 64 0 1 1 0 128 64 64 0 1 1 0-128z"
    />
  ),
  star: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 576 512"
      d="M316.9 18C311.6 7 300.4 0 288.1 0s-23.4 7-28.8 18L195 150.3 51.4 171.5c-12 1.8-22 10.2-25.7 21.7s-.7 24.2 7.9 32.7L137.8 329 113.2 474.7c-2 12 3 24.2 12.9 31.3s23 8 33.8 2.3l128.3-68.5 128.3 68.5c10.8 5.7 23.9 4.9 33.8-2.3s14.9-19.3 12.9-31.3L438.5 329 542.7 225.9c8.6-8.5 11.7-21.2 7.9-32.7s-13.7-19.9-25.7-21.7L381.2 150.3 316.9 18z"
    />
  ),
  pen: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 512 512"
      d="M362.7 19.3L314.3 67.7 444.3 197.7l48.4-48.4c25-25 25-65.5 0-90.5L453.3 19.3c-25-25-65.5-25-90.5 0zm-71 71L58.6 323.5c-10.4 10.4-18 23.3-22.2 37.4L1 481.2C-1.5 489.7 .8 498.8 7 505s15.3 8.5 23.7 6.1l120.3-35.4c14.1-4.2 27-11.8 37.4-22.2L421.7 220.3 291.7 90.3z"
    />
  ),
  trash: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 448 512"
      d="M135.2 17.7L128 32 32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-96 0-7.2-14.3C307.4 6.8 296.3 0 284.2 0L163.8 0c-12.1 0-23.2 6.8-28.6 17.7zM416 128L32 128 53.2 467c1.6 25.3 22.6 45 47.9 45l245.8 0c25.3 0 46.3-19.7 47.9-45L416 128z"
    />
  ),
  plus: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 448 512"
      d="M256 80c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 144L48 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l144 0 0 144c0 17.7 14.3 32 32 32s32-14.3 32-32l0-144 144 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-144 0 0-144z"
    />
  ),
  check: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 448 512"
      d="M438.6 105.4c12.5 12.5 12.5 32.8 0 45.3l-256 256c-12.5 12.5-32.8 12.5-45.3 0l-128-128c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L160 338.7 393.4 105.4c12.5-12.5 32.8-12.5 45.3 0z"
    />
  ),
  ellipsis: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 448 512"
      d="M8 256a56 56 0 1 1 112 0A56 56 0 1 1 8 256zm160 0a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zm216-56a56 56 0 1 1 0 112 56 56 0 1 1 0-112z"
    />
  ),
  phone: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 512 512"
      d="M164.9 24.6c-7.7-18.6-28-28.5-47.4-23.2l-88 24C12.1 30.2 0 46 0 64C0 311.4 200.6 512 448 512c18 0 33.8-12.1 38.6-29.5l24-88c5.3-19.4-4.6-39.7-23.2-47.4l-96-40c-16.3-6.8-35.2-2.1-46.3 11.6L304.7 368C234.3 334.7 177.3 277.7 144 207.3L193.3 167c13.7-11.2 18.4-30 11.6-46.3l-40-96z"
    />
  ),
  external: (c?: string) => (
    <Icon
      className={c}
      viewBox="0 0 512 512"
      d="M320 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l82.7 0L201.4 265.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L448 109.3l0 82.7c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160c0-17.7-14.3-32-32-32L320 0zM80 32C35.8 32 0 67.8 0 112L0 432c0 44.2 35.8 80 80 80l320 0c44.2 0 80-35.8 80-80l0-112c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 112c0 8.8-7.2 16-16 16L80 448c-8.8 0-16-7.2-16-16l0-320c0-8.8 7.2-16 16-16l112 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80 32z"
    />
  ),
}

// ── Sorting ──────────────────────────────────────────────────────────────────

const byName: O.Ord<Place> = O.fromCompare((a, b) => {
  const c = a.name.localeCompare(b.name, 'fr', { sensitivity: 'base' })
  return c < 0 ? -1 : c > 0 ? 1 : 0
})

// Best rated first; places nobody rated yet come last.
const byRating: O.Ord<Place> = pipe(
  N.Ord,
  O.reverse,
  O.contramap((p: Place) => p.average_rating ?? -1),
)

const byGoers: O.Ord<Place> = pipe(
  N.Ord,
  O.reverse,
  O.contramap((p: Place) => p.goers.length),
)

const SORTS: Record<SortKey, O.Ord<Place>[]> = {
  rating: [byRating, byGoers, byName],
  goers: [byGoers, byRating, byName],
  name: [byName],
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const formatRating = (r: number) => r.toLocaleString('fr-FR', { minimumFractionDigits: 1, maximumFractionDigits: 1 })

const goerName = (g: Goer) => (g.last_initial ? `${g.first_name} ${g.last_initial}.` : g.first_name)

const emptyForm = (): PlaceInput => ({
  name: '',
  cuisine: '',
  address: '',
  maps_url: '',
  open_days: [...WEEKDAYS],
  under_ten_euros: false,
  veggie_friendly: false,
  phone: '',
  description: '',
})

const formFrom = (p: Place): PlaceInput => ({
  name: p.name,
  cuisine: p.cuisine,
  address: p.address,
  maps_url: p.maps_url ?? '',
  under_ten_euros: p.under_ten_euros,
  veggie_friendly: p.veggie_friendly,
  phone: p.phone ?? '',
  description: p.description,
  open_days: p.open_days,
})

// ── Components ───────────────────────────────────────────────────────────────

// The band at the top of a card: the icon of the place's kind of cuisine, and its description.
function Cover({ cuisine, description }: { cuisine: string; description: string }) {
  const icon = CUISINE_ICONS[cuisineIcon(cuisine)]
  return (
    <div className="lunch-cover">
      <svg
        viewBox={`0 0 ${icon.width} ${icon.height}`}
        className="h-10 w-auto shrink-0 fill-current"
        aria-hidden="true"
      >
        <path d={icon.path} />
      </svg>
      {description && <p className="lunch-description">{description}</p>}
    </div>
  )
}

function Stars({ value }: { value: number }) {
  return (
    <span className="lunch-stars" aria-hidden="true">
      {[1, 2, 3, 4, 5].map((n) => (
        <span key={n} className={`lunch-star-cell lunch-star ${value >= n - 0.25 ? 'is-on' : ''}`}>
          {icons.star()}
        </span>
      ))}
    </span>
  )
}

function MyRating({
  value,
  onRate,
  disabled,
}: {
  value: number | null
  onRate: (n: number | null) => void
  disabled: boolean
}) {
  const [hover, setHover] = useState<number | null>(null)
  const shown = hover ?? value ?? 0
  return (
    <span className="lunch-stars" role="group" aria-label="Ta note" onMouseLeave={() => setHover(null)}>
      {[1, 2, 3, 4, 5].map((n) => (
        <button
          key={n}
          type="button"
          className={`lunch-star-cell lunch-star-btn lunch-star ${shown >= n ? 'is-on' : ''}`}
          disabled={disabled}
          onMouseEnter={() => setHover(n)}
          onClick={() => onRate(value === n ? null : n)}
          aria-label={value === n ? `Retirer ta note de ${n} sur 5` : `Noter ${n} sur 5`}
          aria-pressed={value === n}
        >
          {icons.star()}
        </button>
      ))}
    </span>
  )
}

function Days({ days, today }: { days: Weekday[]; today: Weekday | null }) {
  return (
    <div className="flex gap-1.5" aria-label={`Ouvert le midi : ${days.map((d) => WEEKDAY_LABELS[d]).join(', ')}`}>
      {WEEKDAYS.map((d) => {
        const open = days.includes(d)
        return (
          <span
            key={d}
            className={`lunch-day ${open ? '' : 'is-closed'} ${d === today ? 'is-today' : ''}`}
            title={open ? 'Ouvert' : 'Fermé'}
            aria-hidden="true"
          >
            {WEEKDAY_LABELS[d]}
          </span>
        )
      })}
    </div>
  )
}

function Goers({ goers, myId }: { goers: Goer[]; myId: number | undefined }) {
  if (goers.length === 0) return null
  return (
    <div className="flex flex-wrap gap-1" aria-label="Y vont ce midi">
      {goers.map((g) => (
        <span key={g.user_id} className={`lunch-goer ${g.user_id === myId ? 'is-me' : ''}`}>
          {g.user_id === myId ? 'Toi' : goerName(g)}
        </span>
      ))}
    </div>
  )
}

// One line of the card's contact block: a fixed-width icon column, then the content.
function InfoRow({ icon, children }: { icon: ReactNode; children: ReactNode }) {
  return (
    <>
      <span className="lunch-blue flex h-5 items-center justify-center" aria-hidden="true">
        {icon}
      </span>
      <div className="min-w-0">{children}</div>
    </>
  )
}

interface CardProps {
  place: Place
  today: Weekday | null
  mine: boolean
  myId: number | undefined
  busy: boolean
  onGo: () => void
  onRate: (n: number | null) => void
  onEdit: () => void
  onDelete: () => void
}

function PlaceCard({ place, today, mine, myId, busy, onGo, onRate, onEdit, onDelete }: CardProps) {
  const closed = today !== null && !place.open_today
  const hasInfo = place.address || place.maps_url || place.phone
  return (
    <article className={`lunch-card flex flex-col ${mine ? 'is-mine' : ''} ${closed ? 'is-closed' : ''}`}>
      <Cover cuisine={place.cuisine} description={place.description} />

      <div className="flex flex-1 flex-col gap-5 p-5">
        <header className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-2">
            <h2 className="lunch-card-name break-words">{place.name}</h2>
            {(place.cuisine || place.under_ten_euros || place.veggie_friendly || closed) && !place.is_gamelle && (
              <div className="flex flex-wrap gap-1.5">
                {place.cuisine && <span className="lunch-tag">{place.cuisine}</span>}
                {place.under_ten_euros && <span className="lunch-tag is-perk">Moins de 10&nbsp;€</span>}
                {place.veggie_friendly && <span className="lunch-tag is-perk">Végé-friendly</span>}
                {closed && <span className="lunch-tag">Fermé aujourd'hui</span>}
              </div>
            )}
          </div>
          {!place.is_gamelle && (
            <div className="dropdown dropdown-end -mr-2 -mt-1 shrink-0">
              <div
                tabIndex={0}
                role="button"
                className="btn btn-ghost btn-sm btn-circle"
                aria-label={`Actions pour ${place.name}`}
              >
                {icons.ellipsis()}
              </div>
              <ul
                tabIndex={0}
                className="dropdown-content menu bg-base-100 rounded-box shadow-lg border border-base-200 z-20 w-40 p-1"
              >
                <li>
                  <button type="button" onClick={onEdit}>
                    {icons.pen()} Modifier
                  </button>
                </li>
                <li>
                  <button type="button" className="text-error" onClick={onDelete}>
                    {icons.trash()} Supprimer
                  </button>
                </li>
              </ul>
            </div>
          )}
        </header>

        {place.is_gamelle ? (
          <p className="text-sm leading-[22px] text-base-content/80">
            Pour celles et ceux qui ont apporté leur déjeuner. On se retrouve à la cuisine !
          </p>
        ) : (
          <>
            {hasInfo && (
              <div className="grid grid-cols-[1rem_1fr] gap-x-3 gap-y-2 text-sm leading-5">
                {place.address && <InfoRow icon={icons.location()}>{place.address}</InfoRow>}
                {place.maps_url && (
                  <InfoRow icon={icons.external('text-xs')}>
                    <a
                      href={place.maps_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="lunch-blue font-bold hover:underline"
                    >
                      Voir sur Maps
                    </a>
                  </InfoRow>
                )}
                {place.phone && (
                  <InfoRow icon={icons.phone('text-xs')}>
                    <a
                      href={`tel:${place.phone.replace(/[^\d+]/g, '')}`}
                      className="group inline-flex flex-wrap gap-x-2"
                      aria-label={`Appeler ${place.name} au ${place.phone}`}
                    >
                      <span className="tabular-nums">{place.phone}</span>
                      <span className="lunch-blue font-bold group-hover:underline">Appeler</span>
                    </a>
                  </InfoRow>
                )}
              </div>
            )}

            <div className="grid grid-cols-[4.5rem_auto_1fr] items-center gap-x-3 gap-y-1.5 text-sm">
              <span className="text-base-content/70">Moyenne</span>
              {place.average_rating !== null ? (
                <>
                  <Stars value={place.average_rating} />
                  <span>
                    <span className="font-bold">{formatRating(place.average_rating)}</span>
                    <span className="text-base-content/70"> · {place.rating_count} avis</span>
                  </span>
                </>
              ) : (
                <span className="col-span-2 text-base-content/70">Pas encore noté</span>
              )}
              <span className="text-base-content/70">Ta note</span>
              <span className="col-span-2">
                <MyRating value={place.my_rating} onRate={onRate} disabled={busy} />
              </span>
            </div>

            <Days days={place.open_days} today={today} />
          </>
        )}

        <footer className="lunch-card-footer mt-auto flex flex-wrap items-center gap-3 pt-4">
          <button
            type="button"
            className={`lunch-btn ${mine ? '' : 'is-outline'}`}
            onClick={onGo}
            disabled={busy || (closed && !mine)}
            aria-pressed={mine}
            title={mine ? 'Cliquer pour changer d’avis' : undefined}
          >
            {mine ? (
              <>
                {icons.check()} {place.is_gamelle ? 'Gamelle ce midi' : "J'y vais"}
              </>
            ) : place.is_gamelle ? (
              "C'est mon cas"
            ) : (
              "J'y vais ce midi"
            )}
          </button>
          <Goers goers={place.goers} myId={myId} />
        </footer>
      </div>
    </article>
  )
}

// ── Page ─────────────────────────────────────────────────────────────────────

export function Lunch() {
  const notify = useToast()
  const myId = getTokenPayload()?.sub

  const [today, setToday] = useState<Today | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const [openOnly, setOpenOnly] = useState(true)
  const [cheapOnly, setCheapOnly] = useState(false)
  const [veggieOnly, setVeggieOnly] = useState(false)
  const [cuisines, setCuisines] = useState<string[]>([])
  const [sort, setSort] = useState<SortKey>('rating')

  const [editing, setEditing] = useState<Place | 'new' | null>(null)
  const [form, setForm] = useState<PlaceInput>(emptyForm)
  const [formError, setFormError] = useState<string | null>(null)
  const [deleting, setDeleting] = useState<Place | null>(null)

  const load = useCallback(async () => {
    try {
      setToday(await fetchToday())
      setLoadError(null)
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Erreur inconnue')
    }
  }, [])

  useEffect(() => {
    load()
    const refresh = () => {
      if (document.visibilityState === 'visible') load()
    }
    const timer = setInterval(refresh, REFRESH_MS)
    document.addEventListener('visibilitychange', refresh)
    return () => {
      clearInterval(timer)
      document.removeEventListener('visibilitychange', refresh)
    }
  }, [load])

  const weekend = today !== null && today.weekday === null

  // Runs a change, then reloads the day; errors are shown as a toast.
  const run = async (action: () => Promise<void>, success?: string) => {
    setBusy(true)
    try {
      await action()
      await load()
      if (success) notify(success, 'success')
      return true
    } catch (e) {
      notify(e instanceof Error ? e.message : 'Erreur inconnue', 'error')
      return false
    } finally {
      setBusy(false)
    }
  }

  const places = today?.places ?? []
  const gamelle = places.find((p) => p.is_gamelle)
  const myPlace = places.find((p) => p.id === today?.my_place_id)

  const shown = useMemo(
    () =>
      pipe(
        places,
        A.filter((p) => !p.is_gamelle),
        A.filter((p) => weekend || !openOnly || p.open_today),
        A.filter((p) => cuisines.length === 0 || cuisines.includes(p.cuisine)),
        A.filter((p) => !cheapOnly || p.under_ten_euros),
        A.filter((p) => !veggieOnly || p.veggie_friendly),
        A.sortBy(SORTS[sort]),
      ),
    [places, weekend, openOnly, cheapOnly, veggieOnly, cuisines, sort],
  )

  const busiest = useMemo(
    () =>
      pipe(
        places,
        A.filter((p) => p.goers.length > 0),
        A.sortBy([byGoers, byName]),
      ),
    [places],
  )

  const toggleCuisine = (c: string) =>
    setCuisines((prev) => (prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c]))

  const go = (place: Place) => run(() => setIntention(today?.my_place_id === place.id ? null : place.id))

  const rate = (place: Place, stars: number | null) => run(() => setRating(place.id, stars))

  const openForm = (target: Place | 'new') => {
    setEditing(target)
    setForm(target === 'new' ? emptyForm() : formFrom(target))
    setFormError(null)
  }

  const submitForm = async (e: FormEvent) => {
    e.preventDefault()
    const parsed = PlaceInputSchema.safeParse(form)
    if (!parsed.success) {
      setFormError(parsed.error.issues[0]?.message ?? 'Formulaire invalide')
      return
    }
    const target = editing
    const ok = await run(
      async () => {
        if (target === 'new') await createPlace(parsed.data)
        else if (target) await updatePlace(target.id, parsed.data)
      },
      target === 'new' ? `« ${parsed.data.name} » est ajouté, bon appétit !` : 'Fiche mise à jour.',
    )
    if (ok) setEditing(null)
  }

  const confirmDelete = async () => {
    if (!deleting) return
    const name = deleting.name
    const ok = await run(() => deletePlace(deleting.id), `« ${name} » a été supprimé.`)
    if (ok) setDeleting(null)
  }

  const toggleDay = (d: Weekday) =>
    setForm((f) => ({
      ...f,
      open_days: f.open_days.includes(d)
        ? f.open_days.filter((x) => x !== d)
        : WEEKDAYS.filter((w) => w === d || f.open_days.includes(w)),
    }))

  const dateLabel = today ? format(parseISO(today.date), 'EEEE d MMMM', { locale: fr }) : ''

  const cardProps = (p: Place) => ({
    place: p,
    today: today?.weekday ?? null,
    mine: today?.my_place_id === p.id,
    myId,
    busy,
    onGo: () => go(p),
    onRate: (n: number | null) => rate(p, n),
    onEdit: () => openForm(p),
    onDelete: () => setDeleting(p),
  })

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="lunch flex-1 px-4 py-8 md:px-8 max-w-6xl mx-auto w-full flex flex-col gap-8">
        <PageHeader
          title="Pause-déj"
          lede={<>On mange où ce midi ?{dateLabel && <span className="font-normal"> · {dateLabel}</span>}</>}
        />

        {loadError && (
          <div role="alert" className="alert alert-error">
            <span>Impossible de charger la pause-déj : {loadError}</span>
            <button type="button" className="btn btn-sm" onClick={load}>
              Réessayer
            </button>
          </div>
        )}

        {!today && !loadError && (
          <div className="flex justify-center py-16">
            <span className="loading loading-spinner loading-lg" aria-label="Chargement" />
          </div>
        )}

        {today && (
          <>
            {/* Who goes where today */}
            <section className="lunch-card p-4 md:p-6 flex flex-col gap-4" aria-labelledby="lunch-today">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h2 id="lunch-today" className="cw-subtitle">
                  Ce midi
                </h2>
                {myPlace ? (
                  <div className="flex flex-wrap items-center gap-3 text-sm">
                    <span>
                      {myPlace.is_gamelle ? (
                        <>
                          Tu as <strong>ta gamelle</strong>.
                        </>
                      ) : (
                        <>
                          Tu vas chez <strong>{myPlace.name}</strong>.
                        </>
                      )}
                    </span>
                    <button
                      type="button"
                      className="lunch-btn is-outline"
                      disabled={busy}
                      onClick={() => run(() => setIntention(null))}
                    >
                      Changer d'avis
                    </button>
                  </div>
                ) : (
                  <span className="text-sm text-base-content/70">
                    Tu n'as pas encore choisi : un clic sur « J'y vais » suffit.
                  </span>
                )}
              </div>
              {busiest.length === 0 ? (
                <p className="text-sm text-base-content/70">
                  {weekend
                    ? "C'est le week-end : on se retrouve lundi !"
                    : 'Personne ne s’est encore lancé. Montre l’exemple !'}
                </p>
              ) : (
                <ul className="grid grid-cols-1 sm:grid-cols-[max-content_1fr] items-center gap-x-6 gap-y-2">
                  {busiest.map((p) => (
                    <li key={p.id} className="contents">
                      <span className="font-bold">
                        {p.is_gamelle ? 'Gamelle' : p.name}
                        <span className="font-normal text-base-content/70"> · {p.goers.length}</span>
                      </span>
                      <Goers goers={p.goers} myId={myId} />
                    </li>
                  ))}
                </ul>
              )}
            </section>

            {/* Filters */}
            <section className="flex flex-col gap-3" aria-label="Filtres">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-2">
                  {!weekend && (
                    <>
                      <button
                        type="button"
                        className="lunch-chip"
                        aria-pressed={openOnly}
                        onClick={() => setOpenOnly(true)}
                      >
                        Ouverts aujourd'hui
                      </button>
                      <button
                        type="button"
                        className="lunch-chip"
                        aria-pressed={!openOnly}
                        onClick={() => setOpenOnly(false)}
                      >
                        Tous
                      </button>
                    </>
                  )}
                  <button
                    type="button"
                    className="lunch-chip"
                    aria-pressed={cheapOnly}
                    onClick={() => setCheapOnly((v) => !v)}
                  >
                    Moins de 10&nbsp;€
                  </button>
                  <button
                    type="button"
                    className="lunch-chip"
                    aria-pressed={veggieOnly}
                    onClick={() => setVeggieOnly((v) => !v)}
                  >
                    Végé-friendly
                  </button>
                  <label className="flex items-center gap-2 text-sm">
                    <span className="text-base-content/70">Trier</span>
                    <select
                      className="select select-sm select-bordered"
                      value={sort}
                      onChange={(e) => setSort(e.target.value as SortKey)}
                    >
                      {(Object.keys(SORT_LABELS) as SortKey[]).map((k) => (
                        <option key={k} value={k}>
                          {SORT_LABELS[k]}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <button type="button" className="lunch-btn" onClick={() => openForm('new')}>
                  {icons.plus()} Ajouter un endroit
                </button>
              </div>
              {today.cuisines.length > 0 && (
                <div className="flex flex-wrap gap-2" aria-label="Types de cuisine">
                  {today.cuisines.map((c) => (
                    <button
                      key={c}
                      type="button"
                      className="lunch-chip"
                      aria-pressed={cuisines.includes(c)}
                      onClick={() => toggleCuisine(c)}
                    >
                      {c}
                    </button>
                  ))}
                  {cuisines.length > 0 && (
                    <button
                      type="button"
                      className="text-sm lunch-blue font-bold hover:underline px-2"
                      onClick={() => setCuisines([])}
                    >
                      Tout afficher
                    </button>
                  )}
                </div>
              )}
            </section>

            {/* Places */}
            <section className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3" aria-label="Les endroits">
              {gamelle && <PlaceCard {...cardProps(gamelle)} />}
              {shown.map((p) => (
                <PlaceCard key={p.id} {...cardProps(p)} />
              ))}
            </section>
            {shown.length === 0 && (
              <p className="text-center text-base-content/70">
                Rien ne correspond à ces filtres. La gamelle, elle, est toujours ouverte.
              </p>
            )}
          </>
        )}
      </main>

      {/* Create / edit */}
      <dialog className={`modal ${editing ? 'modal-open' : ''}`} aria-labelledby="lunch-form-title">
        <div className="modal-box lunch">
          <h3 id="lunch-form-title" className="cw-subtitle mb-4">
            {editing === 'new' ? 'Un nouvel endroit' : 'Modifier la fiche'}
          </h3>
          <form onSubmit={submitForm} className="flex flex-col gap-4" noValidate>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Nom</span>
              <input
                className="input input-bordered w-full"
                value={form.name}
                maxLength={80}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="Chez Victor Hugo"
                required
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Type de cuisine</span>
              <input
                className="input input-bordered w-full"
                value={form.cuisine}
                maxLength={40}
                list="lunch-cuisines"
                onChange={(e) => setForm({ ...form, cuisine: e.target.value })}
                placeholder="Burgers, Libanais, Boulangerie…"
              />
              <datalist id="lunch-cuisines">
                {today?.cuisines.map((c) => (
                  <option key={c} value={c} />
                ))}
              </datalist>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Description (facultatif)</span>
              <textarea
                className="textarea textarea-bordered w-full"
                rows={3}
                maxLength={500}
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
                placeholder="Formule midi à 11 €, terrasse au soleil, mieux vaut appeler avant…"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Adresse</span>
              <input
                className="input input-bordered w-full"
                value={form.address}
                maxLength={200}
                onChange={(e) => setForm({ ...form, address: e.target.value })}
                placeholder="7 rue de Villefranche, 34090 Montpellier"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Lien Google Maps (facultatif)</span>
              <input
                className="input input-bordered w-full"
                type="url"
                value={form.maps_url}
                onChange={(e) => setForm({ ...form, maps_url: e.target.value })}
                placeholder="https://maps.app.goo.gl/…"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-sm text-base-content/80">Téléphone (facultatif)</span>
              <input
                className="input input-bordered w-full"
                type="tel"
                inputMode="tel"
                maxLength={30}
                value={form.phone}
                onChange={(e) => setForm({ ...form, phone: e.target.value })}
                placeholder="04 67 12 34 56"
              />
            </label>
            <fieldset className="flex flex-col gap-2">
              <legend className="text-sm text-base-content/80 mb-1">Ouvert le midi</legend>
              <div className="flex flex-wrap gap-2">
                {WEEKDAYS.map((d) => (
                  <button
                    key={d}
                    type="button"
                    className="lunch-chip"
                    aria-pressed={form.open_days.includes(d)}
                    onClick={() => toggleDay(d)}
                  >
                    {WEEKDAY_LABELS[d]}
                  </button>
                ))}
              </div>
            </fieldset>
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                className="checkbox checkbox-sm"
                checked={form.under_ten_euros}
                onChange={(e) => setForm({ ...form, under_ten_euros: e.target.checked })}
              />
              <span className="text-sm">Possibilité de manger pour moins de 10&nbsp;€</span>
            </label>
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                className="checkbox checkbox-sm"
                checked={form.veggie_friendly}
                onChange={(e) => setForm({ ...form, veggie_friendly: e.target.checked })}
              />
              <span className="text-sm">Végé-friendly : de bonnes options végétariennes</span>
            </label>
            {formError && (
              <div role="alert" className="alert alert-error py-2 text-sm">
                {formError}
              </div>
            )}
            <div className="modal-action mt-2">
              <button type="button" className="btn btn-ghost" onClick={() => setEditing(null)}>
                Annuler
              </button>
              <button type="submit" className="lunch-btn" disabled={busy}>
                {editing === 'new' ? 'Ajouter' : 'Enregistrer'}
              </button>
            </div>
          </form>
        </div>
        <div className="modal-backdrop" onClick={() => setEditing(null)} />
      </dialog>

      {/* Delete confirmation */}
      <dialog className={`modal ${deleting ? 'modal-open' : ''}`} aria-labelledby="lunch-delete-title">
        <div className="modal-box lunch">
          <h3 id="lunch-delete-title" className="cw-subtitle mb-2">
            Supprimer « {deleting?.name} » ?
          </h3>
          <p className="text-sm text-base-content/80">
            La fiche disparaît pour tout le monde, avec ses notes et les choix de ce midi.
          </p>
          <div className="modal-action">
            <button type="button" className="btn btn-ghost" onClick={() => setDeleting(null)}>
              Annuler
            </button>
            <button type="button" className="btn btn-error rounded-full" disabled={busy} onClick={confirmDelete}>
              {icons.trash()} Supprimer
            </button>
          </div>
        </div>
        <div className="modal-backdrop" onClick={() => setDeleting(null)} />
      </dialog>
    </div>
  )
}
