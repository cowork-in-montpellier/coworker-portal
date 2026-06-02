import { addDays, endOfWeek, format, getDay, parse, startOfWeek } from 'date-fns'
import { fr } from 'date-fns/locale'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Calendar, type EventProps, type SlotInfo, dateFnsLocalizer } from 'react-big-calendar'
import 'react-big-calendar/lib/css/react-big-calendar.css'
import '../styles/calendar.css'
import { Navbar } from '../components/Navbar'
import { useToast } from '../lib/toast'
import {
  type Booking,
  type Room,
  createBooking,
  deleteBooking,
  listBookings,
  listRooms,
} from '../api/rooms'
import { ApiError } from '../api/client'
import { getTokenPayload, isAuthenticated } from '../auth'

const MIN_TIME = new Date(1970, 0, 1, 8, 0, 0)
const MAX_TIME = new Date(1970, 0, 1, 20, 0, 0)

const localizer = dateFnsLocalizer({
  format,
  parse,
  startOfWeek: (date: Date) => startOfWeek(date, { locale: fr }),
  getDay,
  locales: { fr },
})

interface CalendarEvent {
  id: number
  title: string
  start: Date
  end: Date
  resourceId: number
  notes: string
}

interface CreateForm {
  room_id: number
  title: string
  date: string
  start_time: string
  end_time: string
  notes: string
}


function rangeToStartEnd(range: Date[] | { start: Date; end: Date }): { start: Date; end: Date } {
  if (Array.isArray(range)) {
    const start = range[0]
    const end = addDays(range[range.length - 1], 1)
    return { start, end }
  }
  return range
}

export function CalendarPage() {
  const toast = useToast()
  const [rooms, setRooms] = useState<Room[]>([])
  const [bookings, setBookings] = useState<Booking[]>([])
  const [rangeStart, setRangeStart] = useState<Date>(() => startOfWeek(new Date(), { locale: fr }))
  const [rangeEnd, setRangeEnd] = useState<Date>(() => addDays(endOfWeek(new Date(), { locale: fr }), 1))

  const [createOpen, setCreateOpen] = useState(false)
  const [createForm, setCreateForm] = useState<CreateForm>({ room_id: 0, title: '', date: '', start_time: '', end_time: '', notes: '' })
  const [creating, setCreating] = useState(false)

  const [selectedBooking, setSelectedBooking] = useState<Booking | null>(null)
  const [deleting, setDeleting] = useState(false)

  const [copied, setCopied] = useState<number | 'all' | null>(null)
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    listRooms()
      .then(setRooms)
      .catch(() => toast('Erreur lors du chargement des salles', 'error'))
  }, [])

  const loadBookings = useCallback(
    (start: Date, end: Date) => {
      listBookings(start, end)
        .then(setBookings)
        .catch(() => toast('Erreur lors du chargement des réservations', 'error'))
    },
    [],
  )

  useEffect(() => {
    loadBookings(rangeStart, rangeEnd)
  }, [rangeStart, rangeEnd, loadBookings])

  const handleRangeChange = (range: Date[] | { start: Date; end: Date }) => {
    const { start, end } = rangeToStartEnd(range)
    setRangeStart(start)
    setRangeEnd(end)
  }

  const handleSelectSlot = (slot: SlotInfo) => {
    const firstRoom = rooms[0]
    const pad = (n: number) => String(n).padStart(2, '0')
    const d = slot.start
    setCreateForm({
      room_id: (slot.resourceId as number | undefined) ?? firstRoom?.id ?? 0,
      title: '',
      date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`,
      start_time: `${pad(d.getHours())}:${pad(d.getMinutes())}`,
      end_time: `${pad(slot.end.getHours())}:${pad(slot.end.getMinutes())}`,
      notes: '',
    })
    setCreateOpen(true)
  }

  const handleSelectEvent = (event: CalendarEvent) => {
    const booking = bookings.find(b => b.id === event.id) ?? null
    setSelectedBooking(booking)
  }

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    setCreating(true)
    try {
      const username = getTokenPayload()?.username ?? ''
      const label = createForm.title.trim()
      const title = label ? `${username} / ${label}` : username
      const start = new Date(`${createForm.date}T${createForm.start_time}`)
      const end = new Date(`${createForm.date}T${createForm.end_time}`)
      await createBooking({
        room_id: createForm.room_id,
        title,
        start_at: start.toISOString(),
        end_at: end.toISOString(),
        notes: createForm.notes || undefined,
      })
      setCreateOpen(false)
      loadBookings(rangeStart, rangeEnd)
      toast('Réservation créée', 'success')
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        toast('Cette salle est déjà réservée sur ce créneau', 'error')
      } else if (err instanceof ApiError) {
        toast(err.message, 'error')
      } else {
        toast('Erreur lors de la création', 'error')
      }
    } finally {
      setCreating(false)
    }
  }

  const handleDelete = async () => {
    if (!selectedBooking) return
    setDeleting(true)
    try {
      await deleteBooking(selectedBooking.id)
      setSelectedBooking(null)
      loadBookings(rangeStart, rangeEnd)
      toast('Réservation supprimée', 'success')
    } catch {
      toast('Erreur lors de la suppression', 'error')
    } finally {
      setDeleting(false)
    }
  }

  const handleCopy = (id: number | 'all', url: string) => {
    navigator.clipboard.writeText(url).then(() => {
      setCopied(id)
      if (copiedTimer.current) clearTimeout(copiedTimer.current)
      copiedTimer.current = setTimeout(() => setCopied(null), 2000)
    })
  }

  const events: CalendarEvent[] = bookings.map(b => ({
    id: b.id,
    title: b.title,
    start: new Date(b.start_at),
    end: new Date(b.end_at),
    resourceId: b.room_id,
    notes: b.notes,
  }))

  const eventPropGetter = (event: CalendarEvent) => {
    const room = rooms.find(r => r.id === event.resourceId)
    const color = room?.color ?? '#3b82f6'
    return { style: { backgroundColor: color, borderColor: color, color: '#fff' } }
  }

  const AgendaEvent = useMemo(
    () =>
      function AgendaEventRow({ event }: EventProps<CalendarEvent>) {
        const [expanded, setExpanded] = useState(false)
        const room = rooms.find(r => r.id === event.resourceId)
        const dashIdx = event.title.indexOf(' - ')
        const creator = dashIdx >= 0 ? event.title.slice(0, dashIdx) : event.title

        return (
          <div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                className="btn btn-ghost btn-xs p-0 min-h-0 h-5 w-5 flex-shrink-0"
                onClick={e => { e.stopPropagation(); setExpanded(v => !v) }}
                aria-label={expanded ? 'Réduire' : 'Détails'}
              >
                <span
                  className="inline-block transition-transform duration-150"
                  style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
                >
                  ▶
                </span>
              </button>
              {room && (
                <span
                  className="badge badge-sm flex-shrink-0"
                  style={{ backgroundColor: room.color, color: '#fff', borderColor: room.color }}
                >
                  {room.name}
                </span>
              )}
              <span>{event.title}</span>
            </div>
            {expanded && (
              <div className="mt-2 ml-7 p-3 rounded-lg bg-base-200 text-sm flex flex-col gap-1.5">
                <div>
                  <span className="font-medium">Salle :</span>{' '}
                  {room ? (
                    <span className="inline-flex items-center gap-1">
                      <span className="inline-block w-2 h-2 rounded-full" style={{ backgroundColor: room.color }} />
                      {room.name}
                    </span>
                  ) : `#${event.resourceId}`}
                </div>
                <div><span className="font-medium">Créé par :</span> {creator}</div>
                <div>
                  <span className="font-medium">Horaire :</span>{' '}
                  {format(event.start, "HH:mm", { locale: fr })} – {format(event.end, "HH:mm", { locale: fr })}
                </div>
                {event.notes && (
                  <div><span className="font-medium">Notes :</span> {event.notes}</div>
                )}
              </div>
            )}
          </div>
        )
      },
    [rooms],
  )

  const selectedRoom = selectedBooking ? rooms.find(r => r.id === selectedBooking.room_id) : null
  const baseUrl = window.location.origin

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />
      <main className="flex-1 p-4 md:p-8 max-w-7xl mx-auto w-full flex flex-col gap-6">
        <h1 className="text-2xl font-bold">Réservation des salles</h1>

        <div className="card bg-base-100 shadow-sm">
          <div className="card-body p-4">
            <Calendar
              localizer={localizer}
              culture="fr"
              events={events}
              resources={rooms.map(r => ({ id: r.id, title: r.name }))}
              resourceIdAccessor="id"
              resourceTitleAccessor="title"
              defaultView="week"
              min={MIN_TIME}
              max={MAX_TIME}
              scrollToTime={new Date()}
              selectable={isAuthenticated()}
              onSelectSlot={isAuthenticated() ? handleSelectSlot : undefined}
              onSelectEvent={handleSelectEvent as (event: object) => void}
              onRangeChange={handleRangeChange}
              eventPropGetter={eventPropGetter as (event: object) => object}
              components={{ agenda: { event: AgendaEvent as React.ComponentType<EventProps<object>> } }}
              style={{ height: 600 }}
              formats={{
                dayFormat: (date, culture, localizer) =>
                  localizer?.format(date, 'EEEE d', culture) ?? '',
              }}
              messages={{
                week: 'Semaine',
                day: 'Jour',
                month: 'Mois',
                today: "Aujourd'hui",
                previous: 'Précédent',
                next: 'Suivant',
                agenda: 'Agenda',
                date: 'Date',
                time: 'Heure',
                event: 'Événement',
                noEventsInRange: 'Aucune réservation sur cette période.',
                showMore: (total: number) => `+${total} de plus`,
                allDay: 'Journée',
              }}
            />
            <div className="flex flex-wrap gap-4 pt-3 border-t border-base-200">
              {rooms.map(room => (
                <div key={room.id} className="flex items-center gap-2">
                  <span className="inline-block w-3 h-3 rounded-full flex-shrink-0" style={{ backgroundColor: room.color }} />
                  <span className="text-sm">{room.name}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="card bg-base-100 shadow-sm">
          <div className="card-body p-4">
            <h2 className="card-title text-base">Liens iCalendar</h2>
            <p className="text-sm text-base-content/60 mb-3">
              Abonnez-vous à ces liens dans Google Calendar, Outlook ou Zimbra pour voir les réservations en temps réel.
            </p>
            <div className="flex flex-col gap-2">
              {rooms.map(room => {
                const url = `${baseUrl}/api/rooms/${room.id}/calendar.ics`
                return (
                  <div key={room.id} className="flex items-center gap-3">
                    <span
                      className="inline-block w-3 h-3 rounded-full flex-shrink-0"
                      style={{ backgroundColor: room.color }}
                    />
                    <span className="text-sm font-medium w-40 flex-shrink-0">{room.name}</span>
                    <code className="text-xs bg-base-200 px-2 py-1 rounded flex-1 truncate">{url}</code>
                    <button
                      className="btn btn-xs btn-ghost"
                      onClick={() => handleCopy(room.id, url)}
                    >
                      {copied === room.id ? 'Copié ✓' : 'Copier'}
                    </button>
                  </div>
                )
              })}
              <div className="flex items-center gap-3">
                <span className="inline-block w-3 h-3 rounded-full flex-shrink-0 bg-base-content/30" />
                <span className="text-sm font-medium w-40 flex-shrink-0">Toutes les salles</span>
                <code className="text-xs bg-base-200 px-2 py-1 rounded flex-1 truncate">{`${baseUrl}/api/calendar.ics`}</code>
                <button
                  className="btn btn-xs btn-ghost"
                  onClick={() => handleCopy('all', `${baseUrl}/api/calendar.ics`)}
                >
                  {copied === 'all' ? 'Copié ✓' : 'Copier'}
                </button>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Create booking modal */}
      <dialog className={`modal ${createOpen ? 'modal-open' : ''}`}>
        <div className="modal-box">
          <h3 className="font-bold text-lg mb-4">Nouvelle réservation</h3>
          <form onSubmit={handleCreate} className="flex flex-col gap-4">
            <div className="form-control">
              <label className="label">
                <span className="label-text">Salle</span>
              </label>
              <select
                className="select select-bordered w-full"
                value={createForm.room_id}
                onChange={e => setCreateForm(f => ({ ...f, room_id: Number(e.target.value) }))}
                required
              >
                {rooms.map(r => (
                  <option key={r.id} value={r.id}>{r.name}</option>
                ))}
              </select>
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text">Date</span></label>
              <input
                type="date"
                className="input input-bordered w-full"
                required
                value={createForm.date}
                onChange={e => setCreateForm(f => ({ ...f, date: e.target.value }))}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="form-control">
                <label className="label"><span className="label-text">Début</span></label>
                <input
                  type="time"
                  className="input input-bordered w-full"
                  required
                  value={createForm.start_time}
                  onChange={e => setCreateForm(f => ({ ...f, start_time: e.target.value }))}
                />
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text">Fin</span></label>
                <input
                  type="time"
                  className="input input-bordered w-full"
                  required
                  value={createForm.end_time}
                  onChange={e => setCreateForm(f => ({ ...f, end_time: e.target.value }))}
                />
              </div>
            </div>
            <div className="form-control">
              <label className="label">
                <span className="label-text">Nom (optionnel)</span>
                <span className="label-text-alt text-sm text-base-content/30">{getTokenPayload()?.username} / Nom</span>
              </label>
              <input
                type="text"
                className="input input-bordered w-full"
                value={createForm.title}
                onChange={e => setCreateForm(f => ({ ...f, title: e.target.value }))}
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text">Notes (optionnel)</span></label>
              <textarea
                className="textarea textarea-bordered w-full"
                rows={2}
                value={createForm.notes}
                onChange={e => setCreateForm(f => ({ ...f, notes: e.target.value }))}
              />
            </div>
            <div className="card-actions justify-end">
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                onClick={() => setCreateOpen(false)}
                disabled={creating}
              >
                Annuler
              </button>
              <button type="submit" className="btn btn-primary btn-sm" disabled={creating}>
                {creating ? <span className="loading loading-spinner loading-xs" /> : 'Réserver'}
              </button>
            </div>
          </form>
        </div>
        <div className="modal-backdrop" onClick={() => setCreateOpen(false)} />
      </dialog>

      {/* Booking detail / delete modal */}
      <dialog className={`modal ${selectedBooking !== null ? 'modal-open' : ''}`}>
        <div className="modal-box">
          <h3 className="font-bold text-lg mb-4">Réservation</h3>
          {selectedBooking && (
            <div className="flex flex-col gap-2 text-sm">
              <div><span className="font-medium">Titre :</span> {selectedBooking.title}</div>
              <div>
                <span className="font-medium">Salle :</span>{' '}
                <span className="inline-flex items-center gap-1">
                  {selectedRoom && (
                    <span
                      className="inline-block w-2.5 h-2.5 rounded-full"
                      style={{ backgroundColor: selectedRoom.color }}
                    />
                  )}
                  {selectedRoom?.name ?? `Salle #${selectedBooking.room_id}`}
                </span>
              </div>
              <div>
                <span className="font-medium">Début :</span>{' '}
                {format(new Date(selectedBooking.start_at), "EEEE d MMMM yyyy 'à' HH:mm", { locale: fr })}
              </div>
              <div>
                <span className="font-medium">Fin :</span>{' '}
                {format(new Date(selectedBooking.end_at), "EEEE d MMMM yyyy 'à' HH:mm", { locale: fr })}
              </div>
              {selectedBooking.notes && (
                <div><span className="font-medium">Notes :</span> {selectedBooking.notes}</div>
              )}
            </div>
          )}
          <div className="modal-action">
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setSelectedBooking(null)}
              disabled={deleting}
            >
              Fermer
            </button>
            {isAuthenticated() && (
              <button
                type="button"
                className="btn btn-error"
                onClick={handleDelete}
                disabled={deleting}
              >
                {deleting ? <span className="loading loading-spinner loading-xs" /> : 'Supprimer'}
              </button>
            )}
          </div>
        </div>
        <div className="modal-backdrop" onClick={() => setSelectedBooking(null)} />
      </dialog>
    </div>
  )
}
