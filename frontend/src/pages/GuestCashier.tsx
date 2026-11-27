import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { PageHeader } from '../components/PageHeader'
import {
  type PendingOrderItem,
  listPendingOrders,
  validateGuestOrder,
} from '../api/guest'
import { ApiError } from '../api/client'

const AUTO_REFRESH_MS = 15000

interface OrderCardProps {
  order: PendingOrderItem
}

function timeAgo(isoDate: string): string {
  const diffMs = Date.now() - new Date(isoDate).getTime()
  const mins = Math.floor(diffMs / 60000)
  if (mins < 1) return "à l'instant"
  if (mins < 60) return `il y a ${mins} min`
  const hrs = Math.floor(mins / 60)
  return `il y a ${hrs}h`
}

function OrderCard({ order }: OrderCardProps) {
  const [state, setState] = useState<'idle' | 'loading' | 'done' | 'error'>('idle')
  const [guestToken, setGuestToken] = useState<string | null>(null)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const handleValidate = async () => {
    setState('loading')
    setErrorMsg(null)
    try {
      const result = await validateGuestOrder(order.order_token)
      setGuestToken(result.guest_token)
      setState('done')
    } catch (e) {
      const msg =
        e instanceof ApiError && e.status === 400
          ? 'Cette commande a déjà été réglée.'
          : 'Erreur lors de la validation. Veuillez réessayer.'
      setErrorMsg(msg)
      setState('error')
    }
  }

  if (state === 'done' && guestToken) {
    return (
      <div className="card bg-success/10 border-2 border-success/30">
        <div className="card-body p-4 gap-2">
          <div className="flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-success h-5 w-5 shrink-0">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span className="font-semibold text-sm text-success">Paiement validé</span>
          </div>
          <p className="font-mono text-xl font-bold text-base-content/70">{order.provisional_number}</p>
          <Link
            to={`/buy/summary/${guestToken}`}
            className="btn btn-sm btn-ghost text-xs"
          >
            Voir la facture →
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="card cw-card">
      <div className="card-body p-4 gap-3">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <p className="font-mono text-2xl font-bold text-primary tracking-wider">
              {order.provisional_number}
            </p>
            <p className="text-xs text-base-content/40">{timeAgo(order.created_at)}</p>
          </div>
          <div className="text-right shrink-0">
            <p className="text-xl font-bold">{order.total_amount.toFixed(2)} €</p>
          </div>
        </div>

        <p className="text-xs text-base-content/50">{order.guest_email}</p>

        {state === 'error' && errorMsg && (
          <div role="alert" className="alert alert-error py-2 px-3 text-xs">
            <span>{errorMsg}</span>
          </div>
        )}

        <button
          type="button"
          className="btn btn-primary btn-sm w-full"
          disabled={state === 'loading'}
          onClick={handleValidate}
        >
          {state === 'loading'
            ? <span className="loading loading-spinner loading-xs" />
            : 'Valider le paiement'}
        </button>
      </div>
    </div>
  )
}

export function GuestCashier() {
  const [orders, setOrders] = useState<PendingOrderItem[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const fetchOrders = useCallback(async (silent = false) => {
    if (!silent) setRefreshing(true)
    try {
      const data = await listPendingOrders()
      setOrders(data)
      setLastUpdated(new Date())
    } catch {
      // keep existing list visible on transient errors
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }, [])

  useEffect(() => {
    fetchOrders()
    intervalRef.current = setInterval(() => fetchOrders(true), AUTO_REFRESH_MS)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [fetchOrders])

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <PageHeader
          title="Paiements sur place en attente"
          lede={
            lastUpdated &&
            `Mis à jour à ${lastUpdated.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}`
          }
          actions={
            <button type="button" className="btn btn-sm btn-ghost" disabled={refreshing} onClick={() => fetchOrders()}>
              {refreshing ? <span className="loading loading-spinner loading-xs" /> : '↻ Actualiser'}
            </button>
          }
          className="mb-8"
        />

        {loading ? (
          <div className="flex justify-center py-12">
            <span className="loading loading-spinner loading-lg" />
          </div>
        ) : orders.length === 0 ? (
          <div className="card cw-card">
            <div className="card-body p-8 items-center text-center gap-2">
              <p className="text-base-content/40 text-sm">Aucune commande en attente</p>
              <p className="text-base-content/30 text-xs">Les nouvelles commandes apparaissent ici automatiquement.</p>
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {orders.map(order => (
              <OrderCard key={order.order_token} order={order} />
            ))}
          </div>
        )}
      </main>
    </div>
  )
}
