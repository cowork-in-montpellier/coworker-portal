import { useEffect, useState } from 'react'
import { type ConnectedGuestsResponse, fetchConnected } from '../api/connected'
import { fetchOccupancy, type OccupancyResponse, type OccupancySlot } from '../api/occupancy'
import { Navbar } from '../components/Navbar'

// ── Heatmap helpers ───────────────────────────────────────────────────────────

const DAYS = ['Lun', 'Mar', 'Mer', 'Jeu', 'Ven']
const DAY_NUMBERS = [1, 2, 3, 4, 5]
const HOURS = [8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]

const WEEKDAY_MAP: Record<string, number> = {
  Monday: 1, Tuesday: 2, Wednesday: 3, Thursday: 4, Friday: 5,
}

function getCurrentParisSlot(): { day: number; hour: number } | null {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: 'Europe/Paris',
    weekday: 'long',
    hour: 'numeric',
    hour12: false,
  }).formatToParts(new Date())
  const weekday = parts.find(p => p.type === 'weekday')?.value ?? ''
  const hour = parseInt(parts.find(p => p.type === 'hour')?.value ?? '', 10)
  const day = WEEKDAY_MAP[weekday]
  console.log('[occupancy] getCurrentParisSlot', { parts, weekday, hour, day, result: (!day || hour < 8 || hour > 20) ? null : { day, hour } })
  if (!day || hour < 8 || hour > 20) return null
  return { day, hour }
}

function cellColor(value: number, max: number): string {
  if (max === 0 || value === 0) return 'transparent'
  const ratio = value / max
  const lightness = Math.round(80 - ratio * 50)
  return `hsl(221 83% ${lightness}%)`
}

function HeatmapCell({ slot, maxValue, liveCount }: {
  slot: OccupancySlot | undefined
  maxValue: number
  liveCount?: number
}) {
  if (!slot) {
    if (liveCount !== undefined && liveCount > 0) {
      const effectiveMax = Math.max(maxValue, liveCount)
      const bg = cellColor(liveCount, effectiveMax)
      const light = liveCount / effectiveMax <= 0.5
      return (
        <div
          className={`rounded h-9 flex items-center justify-center text-xs italic ${light ? 'text-base-content/70' : 'text-white/80'}`}
          style={{ backgroundColor: bg }}
          title={`~${liveCount} connecté${liveCount !== 1 ? 's' : ''} (créneau en cours)`}
        >
          {liveCount}
        </div>
      )
    }
    return <div className="rounded h-9 bg-base-200 opacity-30" title="Aucune donnée" />
  }
  const bg = cellColor(slot.count, maxValue)
  const light = maxValue === 0 || slot.count / maxValue <= 0.5
  return (
    <div
      className={`rounded h-9 flex items-center justify-center text-xs font-semibold ${light ? 'text-base-content' : 'text-white'}`}
      style={{ backgroundColor: bg }}
      title={`${slot.count} connecté${slot.count > 1 ? 's' : ''}`}
    >
      {slot.count > 0 ? slot.count : ''}
    </div>
  )
}

// ── Page ─────────────────────────────────────────────────────────────────────

export function Live() {
  const [connected, setConnected] = useState<ConnectedGuestsResponse | null>(null)
  const [connectedLoading, setConnectedLoading] = useState(true)

  const [occupancy, setOccupancy] = useState<OccupancyResponse | null>(null)
  const [occupancyLoading, setOccupancyLoading] = useState(true)

  useEffect(() => {
    fetchConnected()
      .then(setConnected)
      .catch(() => { /* non-fatal */ })
      .finally(() => setConnectedLoading(false))

    fetchOccupancy()
      .then(setOccupancy)
      .catch(() => { /* non-fatal */ })
      .finally(() => setOccupancyLoading(false))
  }, [])

  const slotMap = new Map<string, OccupancySlot>()
  occupancy?.slots.forEach(s => slotMap.set(`${s.day}-${s.hour}`, s))
  const maxValue = occupancy?.max_value ?? 0
  const currentParisSlot = getCurrentParisSlot()

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-6xl mx-auto w-full flex flex-col gap-6">
        <h2 className="text-xl font-bold">Le cowo en temps réel</h2>

        {/* Connected now */}
        <div className="card bg-base-100 shadow-sm">
          <div className="card-body p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold text-sm">Connectés en ce moment</h3>
              {connectedLoading && <span className="loading loading-spinner loading-xs text-base-content/40" />}
            </div>

            {!connectedLoading && connected === null && (
              <p className="text-sm text-base-content/40">Impossible de charger les données.</p>
            )}

            {!connectedLoading && connected !== null && connected.total === 0 && (
              <p className="text-sm text-base-content/40">Aucun utilisateur connecté actuellement.</p>
            )}

            {!connectedLoading && connected !== null && connected.total > 0 && (
              <div className="flex flex-wrap gap-4">
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold">{connected.total}</span>
                  <span className="text-sm text-base-content/60">connecté{connected.total !== 1 ? 's' : ''}</span>
                </div>
                <div className="flex items-center gap-3 text-sm text-base-content/60">
                  <span>
                    <span className="font-medium text-base-content">{connected.account_users.length}</span>
                    {' '}membre{connected.account_users.length !== 1 ? 's' : ''}
                  </span>
                  <span>·</span>
                  <span>
                    <span className="font-medium text-base-content">{connected.guest_count}</span>
                    {' '}invité{connected.guest_count !== 1 ? 's' : ''}
                  </span>
                  {connected.unknown_count > 0 && (
                    <>
                      <span>·</span>
                      <span className="text-base-content/40">
                        {connected.unknown_count} essai{connected.unknown_count !== 1 ? 's' : ''}
                      </span>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Occupancy heatmap */}
        <div className="card bg-base-100 shadow-sm overflow-x-auto">
          <div className="card-body p-4 md:p-6">
            <div className="flex items-center justify-between mb-4">
              <div>
                <h3 className="font-semibold text-sm">Occupation — semaine en cours</h3>
                <p className="text-xs text-base-content/40 mt-0.5">
                  Vouchers connectés par créneau cette semaine (lun–ven)
                </p>
              </div>
              {occupancyLoading && <span className="loading loading-spinner loading-xs text-base-content/40" />}
            </div>

            {!occupancyLoading && occupancy !== null && occupancy.slots.length === 0 && (
              <p className="text-sm text-base-content/40 py-4 text-center">
                Aucune donnée cette semaine — les snapshots sont collectés automatiquement du lundi au vendredi, 9h–21h.
              </p>
            )}

            {!occupancyLoading && occupancy !== null && occupancy.slots.length > 0 && (
              <>
                <div
                  className="grid gap-1 min-w-[28rem]"
                  style={{ gridTemplateColumns: '3.5rem repeat(5, 1fr)' }}
                >
                  {/* Header */}
                  <div />
                  {DAYS.map(d => (
                    <div key={d} className="text-center text-xs font-semibold text-base-content/50 uppercase tracking-wide pb-1">
                      {d}
                    </div>
                  ))}

                  {/* Rows */}
                  {HOURS.map(hour => (
                    <>
                      <div key={`label-${hour}`} className="flex items-center justify-end pr-2">
                        <span className="text-xs text-base-content/40 whitespace-nowrap">{hour}h</span>
                      </div>
                      {DAY_NUMBERS.map(day => (
                        <HeatmapCell
                          key={`${day}-${hour}`}
                          slot={slotMap.get(`${day}-${hour}`)}
                          maxValue={maxValue}
                          liveCount={
                            currentParisSlot?.day === day && currentParisSlot?.hour === hour
                              ? (connected?.total ?? 0)
                              : undefined
                          }
                        />
                      ))}
                    </>
                  ))}
                </div>

                {/* Legend */}
                <div className="flex items-center gap-3 mt-4 pt-3 border-t border-base-200">
                  <span className="text-xs text-base-content/40">Calme</span>
                  <div className="flex gap-0.5 flex-1 max-w-28">
                    {[0.1, 0.3, 0.5, 0.7, 0.9].map(r => (
                      <div
                        key={r}
                        className="h-3 flex-1 rounded-sm"
                        style={{ backgroundColor: cellColor(r, 1) }}
                      />
                    ))}
                  </div>
                  <span className="text-xs text-base-content/40">Animé</span>
                  <span className="text-xs text-base-content/30 ml-auto">max {maxValue}</span>
                </div>
              </>
            )}
          </div>
        </div>
      </main>
    </div>
  )
}
