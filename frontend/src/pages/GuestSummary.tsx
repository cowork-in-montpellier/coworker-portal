import { useEffect, useRef, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { Navbar } from '../components/Navbar'
import {
  type GuestBillResponse,
  checkGuestVouchers,
  downloadGuestBillPdf,
  getGuestBill,
  getGuestPaymentStatus,
  switchToCardPayment,
} from '../api/guest'
import type { VoucherStatusEntry } from '../api/bills'
import { generateVoucherPdf } from '../components/VoucherPdf'
import { useStatus } from '../hooks/useStatus'

// ── Payment sub-components ────────────────────────────────────────────────────

function CardPaymentWaiting({ token, onPaid }: { token: string; onPaid: () => void }) {
  const [confirmed, setConfirmed] = useState(false)
  const [exhausted, setExhausted] = useState(false)
  const attemptsRef = useRef(0)
  const MAX_ATTEMPTS = 75

  useEffect(() => {
    const interval = setInterval(async () => {
      attemptsRef.current++
      try {
        const b = await getGuestBill(token)
        if (b.is_paid) {
          setConfirmed(true)
          onPaid()
          clearInterval(interval)
          return
        }
      } catch { /* ignore poll errors */ }
      if (attemptsRef.current >= MAX_ATTEMPTS) {
        setExhausted(true)
        clearInterval(interval)
      }
    }, 4000)
    return () => clearInterval(interval)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token])

  if (confirmed) {
    return (
      <div role="alert" className="alert alert-success alert-soft">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-success h-6 w-6 shrink-0">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span className="text-sm font-bold text-base-content/70">Paiement confirmé, merci !</span>
      </div>
    )
  }
  if (exhausted) {
    return (
      <div role="alert" className="alert alert-warning">
        <div>
          <p className="font-semibold text-sm">Paiement non confirmé</p>
          <p className="text-sm mt-1">Nous n'avons pas encore reçu la confirmation bancaire. Merci de vous rapprocher d'un coworker.</p>
        </div>
      </div>
    )
  }
  return (
    <div className="flex items-center gap-3">
      <span className="loading loading-spinner loading-sm text-primary" />
      <p className="text-sm text-base-content/60">En attente de la confirmation bancaire…</p>
    </div>
  )
}

function OnsitePaymentWaiting({
  token,
  checkoutFailed,
  onPaid,
}: {
  token: string
  checkoutFailed: boolean
  onPaid: () => void
}) {
  const [confirmed, setConfirmed] = useState(false)
  const [exhausted, setExhausted] = useState(false)
  const [switching, setSwitching] = useState(false)
  const [switchError, setSwitchError] = useState<string | null>(null)
  const attemptsRef = useRef(0)
  const MAX_ATTEMPTS = 150

  useEffect(() => {
    const interval = setInterval(async () => {
      attemptsRef.current++
      try {
        const status = await getGuestPaymentStatus(token)
        if (status.paid) {
          setConfirmed(true)
          onPaid()
          clearInterval(interval)
          return
        }
      } catch { /* ignore poll errors */ }
      if (attemptsRef.current >= MAX_ATTEMPTS) {
        setExhausted(true)
        clearInterval(interval)
      }
    }, 4000)
    return () => clearInterval(interval)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token])

  const handleSwitchToCard = async () => {
    setSwitching(true)
    setSwitchError(null)
    try {
      const { payment_url } = await switchToCardPayment(token)
      window.location.href = payment_url
    } catch {
      setSwitchError("Le paiement par carte n'est pas disponible pour le moment.")
      setSwitching(false)
    }
  }

  if (confirmed) {
    return (
      <div role="alert" className="alert alert-success alert-soft">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-success h-6 w-6 shrink-0">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span className="text-sm font-bold text-base-content/70">Paiement confirmé, merci !</span>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-3">
      {checkoutFailed && (
        <div role="alert" className="alert alert-warning py-2 px-3">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-warning h-5 w-5 shrink-0">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <p className="text-sm">Une erreur s'est produite lors du paiement par carte. Votre commande est confirmée — merci de régler sur place auprès d'un coworker.</p>
        </div>
      )}
      {exhausted ? (
        <div role="alert" className="alert alert-info alert-soft">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-info h-6 w-6 shrink-0">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span className="text-sm font-bold text-base-content/50">Le délai d'attente est écoulé. Merci de vous rapprocher d'un coworker pour effectuer le paiement.</span>
        </div>
      ) : (
        <div className="flex items-center gap-3">
          <span className="loading loading-spinner loading-sm text-primary" />
          <p className="text-sm text-base-content/60">Trouvez un coworker pour régler votre facture.</p>
        </div>
      )}
      {switchError && (
        <div role="alert" className="alert alert-error py-2 px-3">
          <span className="text-sm">{switchError}</span>
        </div>
      )}
      <button
        type="button"
        className="btn btn-outline btn-sm w-full"
        disabled={switching}
        onClick={handleSwitchToCard}
      >
        {switching
          ? <span className="loading loading-spinner loading-xs" />
          : '💳 Payer par carte en ligne'}
      </button>
    </div>
  )
}

function PaymentPanel({
  bill,
  token,
  onPaid,
}: {
  bill: GuestBillResponse
  token: string
  onPaid: () => void
}) {
  if (bill.is_paid) {
    return (
      <div role="alert" className="alert alert-success alert-soft">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" className="stroke-success h-6 w-6 shrink-0">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span className="text-sm font-bold text-base-content/70">Paiement confirmé, merci !</span>
      </div>
    )
  }
  if (bill.payment_method === 'card') {
    return <CardPaymentWaiting token={token} onPaid={onPaid} />
  }
  return <OnsitePaymentWaiting token={token} checkoutFailed={bill.checkout_failed} onPaid={onPaid} />
}

// ── Main component ────────────────────────────────────────────────────────────

export function GuestSummary() {
  const { token } = useParams<{ token: string }>()
  const [bill, setBill] = useState<GuestBillResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [voucherStatuses, setVoucherStatuses] = useState<Map<string, string>>(new Map())
  const [checkingVouchers, setCheckingVouchers] = useState(false)
  const [downloadingPdf, setDownloadingPdf] = useState(false)
  const [downloadingInvoice, setDownloadingInvoice] = useState(false)

  useEffect(() => {
    if (!token) return
    getGuestBill(token)
      .then(b => {
        setBill(b)
        const statusMap = new Map<string, string>()
        for (const line of b.lines) {
          for (const v of line.vouchers) {
            statusMap.set(v.unify_id, v.status)
          }
        }
        setVoucherStatuses(statusMap)
      })
      .catch(e => {
        const is404 = e instanceof Error && e.message.includes('404')
        setError(is404 ? '404' : 'Impossible de charger la facture.')
      })
      .finally(() => setLoading(false))
  }, [token])

  const handleCheckVouchers = async () => {
    if (!token) return
    setCheckingVouchers(true)
    try {
      const entries: VoucherStatusEntry[] = await checkGuestVouchers(token)
      setVoucherStatuses(new Map(entries.map(e => [e.unify_id, e.status])))
    } catch {
      // silently ignore — stale statuses stay visible
    } finally {
      setCheckingVouchers(false)
    }
  }

  const handleDownloadVoucherPdf = async () => {
    if (!bill) return
    setDownloadingPdf(true)
    try {
      const entries: VoucherStatusEntry[] = bill.lines.flatMap(line =>
        line.vouchers.map(v => ({
          unify_id: v.unify_id,
          code: v.code,
          duration: v.duration,
          status: voucherStatuses.get(v.unify_id) ?? v.status,
        }))
      )
      await generateVoucherPdf(bill.bill_number, entries)
    } catch {
      // silently ignore
    } finally {
      setDownloadingPdf(false)
    }
  }

  const handleDownloadInvoice = async () => {
    if (!bill || !token) return
    setDownloadingInvoice(true)
    try {
      await downloadGuestBillPdf(token, bill.bill_number)
    } catch {
      // silently ignore
    } finally {
      setDownloadingInvoice(false)
    }
  }

  const { invoice_available } = useStatus()
  const allVouchers = bill?.lines.flatMap(l => l.vouchers) ?? []
  const hasValidVoucher = allVouchers.some(
    v => (voucherStatuses.get(v.unify_id) ?? v.status) === 'Valid'
  )

  if (loading) {
    return (
      <div className="min-h-screen bg-base-200 flex items-center justify-center">
        <span className="loading loading-spinner loading-lg" />
      </div>
    )
  }

  if (error || !bill) {
    const isCancelled = error === '404'
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <Navbar />
        <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
          <div role="alert" className={`alert ${isCancelled ? 'alert-warning' : 'alert-error'}`}>
            <div>
              <p className="font-semibold">
                {isCancelled ? 'Commande annulée' : 'Erreur'}
              </p>
              <p className="text-sm">
                {isCancelled
                  ? 'Le paiement a été annulé et la commande a été supprimée.'
                  : (error ?? 'Facture introuvable.')}
              </p>
              {isCancelled && (
                <Link to="/buy" className="btn btn-sm btn-outline mt-2">Recommencer</Link>
              )}
            </div>
          </div>
        </main>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <div className="mb-6">
          <h2 className="text-xl font-bold">Achat confirmé</h2>
          <p className="text-sm text-base-content/50 mt-1">
            Conservez cette page ou notez vos codes — elle n'est accessible que via ce lien.
          </p>
        </div>

        {/* Bill summary card */}
        <div className="card bg-base-100 shadow-sm mb-4">
          <div className="card-body p-4 gap-2">
            <div className="flex items-start justify-between">
              <div>
                <p className="font-mono font-semibold">{bill.bill_number}</p>
                <p className="text-sm text-base-content/60">
                  {bill.lines
                    .filter(l => l.service_name)
                    .map(l => l.quantity > 1 ? `${l.quantity}× ${l.service_name}` : l.service_name)
                    .join(', ')}
                </p>
                <p className="text-xs text-base-content/40 mt-0.5">{bill.date}</p>
              </div>
              <div className="text-right">
                <p className="text-xl font-bold text-primary">{bill.amount.toFixed(2)} €</p>
                {invoice_available && (
                  <button
                    className="btn btn-xs btn-ghost mt-1"
                    disabled={downloadingInvoice}
                    onClick={handleDownloadInvoice}
                    title="Télécharger la facture PDF"
                  >
                    {downloadingInvoice
                      ? <span className="loading loading-spinner loading-xs" />
                      : '⎙ Facture PDF'}
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Payment panel */}
        {token && (
          <div className="card bg-base-100 shadow-sm mb-4">
            <div className="card-body p-4">
              <PaymentPanel
                bill={bill}
                token={token}
                onPaid={() => setBill(b => b ? { ...b, is_paid: true } : b)}
              />
            </div>
          </div>
        )}

        {/* Vouchers */}
        <div className="card bg-base-100 shadow-sm">
          <div className="card-body p-4">
            <div className="flex items-center gap-2 mb-4">
              <span className="text-xs text-base-content/40 font-medium uppercase tracking-wide">
                Vouchers
              </span>
              <button
                className="btn btn-xs btn-ghost btn-circle"
                disabled={checkingVouchers}
                onClick={handleCheckVouchers}
                title="Vérifier le statut"
              >
                {checkingVouchers
                  ? <span className="loading loading-spinner loading-xs" />
                  : '↻'}
              </button>
              {hasValidVoucher && (
                <button
                  className="btn btn-xs btn-ghost btn-circle"
                  disabled={downloadingPdf}
                  onClick={handleDownloadVoucherPdf}
                  title="Télécharger le PDF des vouchers"
                >
                  {downloadingPdf
                    ? <span className="loading loading-spinner loading-xs" />
                    : '⎙'}
                </button>
              )}
            </div>

            <div className="flex flex-wrap gap-3">
              {allVouchers.map((v, i) => {
                const status = voucherStatuses.get(v.unify_id) ?? v.status
                const isExpired = status === 'Expired' || status === 'Used'
                return (
                  <div
                    key={v.unify_id}
                    className={`card border shadow-sm w-44 transition-opacity ${
                      isExpired
                        ? 'bg-base-200 border-base-300 opacity-40'
                        : 'bg-base-100 border-base-300'
                    }`}
                  >
                    <div className="card-body p-3 gap-1">
                      <div className="flex items-center justify-between">
                        <p className="text-xs text-base-content/40 font-medium">Voucher {i + 1}</p>
                        <span className={`badge badge-xs ${
                          status === 'Valid' ? 'badge-success' :
                          status === 'Used' ? 'badge-neutral' :
                          status === 'Expired' ? 'badge-error' :
                          'badge-ghost'
                        }`}>
                          {status === 'Valid' ? 'Valide' :
                           status === 'Used' ? 'Utilisé' :
                           status === 'Expired' ? 'Expiré' : 'Inconnu'}
                        </span>
                      </div>
                      <p className={`font-mono font-semibold text-sm tracking-wide ${isExpired ? 'line-through' : ''}`}>
                        {v.code}
                      </p>
                      <p className="text-xs text-base-content/50">{v.duration}h</p>
                      {v.active_days_count > 0 && (
                        <p className="text-xs text-primary/70 font-medium">{v.active_days_count} jour{v.active_days_count > 1 ? 's' : ''} actif{v.active_days_count > 1 ? 's' : ''}</p>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
