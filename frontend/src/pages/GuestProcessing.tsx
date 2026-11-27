import { useEffect, useRef, useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import { PageHeader } from '../components/PageHeader'
import { getGuestOrder, getGuestOrderPaymentStatus, type GuestOrderData } from '../api/guest'
import { ApiError } from '../api/client'

const POLL_INTERVAL_MS = 4000
const MAX_POLL_ATTEMPTS = 150 // ~10 minutes

export function GuestProcessing() {
  const { orderToken } = useParams<{ orderToken: string }>()
  const navigate = useNavigate()

  const [order, setOrder] = useState<GuestOrderData | null>(null)
  const [notFound, setNotFound] = useState(false)
  const [copied, setCopied] = useState(false)
  const attemptsRef = useRef(0)

  useEffect(() => {
    if (!orderToken) return
    getGuestOrder(orderToken)
      .then(setOrder)
      .catch(e => {
        if (e instanceof ApiError && e.status === 404) setNotFound(true)
      })
  }, [orderToken])

  useEffect(() => {
    if (!orderToken || notFound) return

    const interval = setInterval(async () => {
      attemptsRef.current++
      try {
        const status = await getGuestOrderPaymentStatus(orderToken)
        if (status.paid && status.guest_token) {
          clearInterval(interval)
          navigate(`/buy/summary/${status.guest_token}`, { replace: true })
          return
        }
      } catch { /* ignore poll errors */ }
      if (attemptsRef.current >= MAX_POLL_ATTEMPTS) {
        clearInterval(interval)
      }
    }, POLL_INTERVAL_MS)

    return () => clearInterval(interval)
  }, [orderToken, notFound, navigate])

  const handleCopy = () => {
    if (!order) return
    navigator.clipboard.writeText(order.provisional_number).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  if (notFound) {
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <Navbar />
        <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
          <div role="alert" className="alert alert-warning">
            <div>
              <p className="font-semibold">Commande introuvable ou expirée</p>
              <p className="text-sm mt-1">Ce lien n'est plus valide.</p>
              <Link to="/buy" className="btn btn-sm btn-outline mt-2">Recommencer</Link>
            </div>
          </div>
        </main>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full flex flex-col items-center justify-center gap-8 py-12">

        <PageHeader title="Paiement" />

        {/* Spinner + status text */}
        <div className="flex flex-col items-center gap-3 text-center">
          <span className="loading loading-spinner loading-lg text-primary" />
          <p className="text-base text-base-content/70">En attente du paiement…</p>
        </div>

        {/* On-site payment info box */}
        {order?.payment_method === 'on_site' && (
          <div className="card cw-card w-full max-w-md">
            <div className="card-body p-6 gap-4 items-center text-center">
              <div className="flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-info h-5 w-5 shrink-0">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span className="text-sm font-semibold text-base-content/70">Paiement sur place</span>
              </div>

              <p className="text-sm text-base-content/60">
                Montrez ce numéro à un coworker disposant d'un terminal de paiement — il devra le saisir sur le terminal SumUp.
              </p>

              <button
                type="button"
                className="font-mono text-3xl font-bold tracking-widest text-primary select-all cursor-copy px-4 py-3 rounded-xl bg-primary/10 hover:bg-primary/20 active:scale-95 transition-all"
                onClick={handleCopy}
                title="Copier le numéro"
              >
                {order.provisional_number}
              </button>

              {copied && (
                <span className="text-xs text-success font-medium">Copié !</span>
              )}

              {order.total_amount > 0 && (
                <p className="text-sm text-base-content/50">
                  Montant à régler&nbsp;: <span className="font-semibold text-base-content/70">{order.total_amount.toFixed(2)} €</span>
                </p>
              )}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
