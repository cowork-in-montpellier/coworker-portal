import { useEffect, useState } from 'react'
import { Navigate, useLocation, useNavigate } from 'react-router-dom'
import { type Service } from '../api/services'
import { createGuestBill } from '../api/guest'
import { ApiError } from '../api/client'
import { useStatus } from '../hooks/useStatus'
import { Navbar } from '../components/Navbar'

interface CheckoutState {
  lines: { service_id: number; quantity: number }[]
  selectedServices: Service[]
  billingName?: string
  billingAddress?: string
  total: number
}

export function GuestCheckout() {
  const location = useLocation()
  const navigate = useNavigate()
  const { sumup_available } = useStatus()
  const routeState = location.state as CheckoutState | null

  // Default to on_site; switch to card once status confirms SumUp is available
  const [paymentMethod, setPaymentMethod] = useState<'card' | 'on_site'>('on_site')
  useEffect(() => {
    if (sumup_available) setPaymentMethod('card')
  }, [sumup_available])
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  if (!routeState) return <Navigate to="/buy" replace />

  const { lines, selectedServices, billingName, billingAddress, total } = routeState

  const handlePay = async () => {
    setSubmitting(true)
    setError(null)
    try {
      const result = await createGuestBill({
        lines,
        billing_name: billingName,
        billing_address: billingAddress,
        payment_method: paymentMethod,
      })
      if (result.payment_url) {
        window.location.href = result.payment_url
      } else {
        navigate(`/buy/summary/${result.guest_token}`)
      }
    } catch (e) {
      const reason =
        e instanceof ApiError && e.status === 502
          ? 'Erreur lors de la création des vouchers. Merci de réessayer plus tard ou de contacter #commission-informatique sur Slack.'
          : 'Veuillez réessayer.'
      setError(`Erreur à la création de la facture : ${reason}`)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-5xl mx-auto w-full">
        <div className="mb-6">
          <h2 className="text-xl font-bold">Récapitulatif & paiement</h2>
        </div>

        {error && (
          <div role="alert" className="alert alert-error mb-4">
            <span>{error}</span>
          </div>
        )}

        <div className="grid md:grid-cols-2 gap-6">
          {/* Left column — order recap */}
          <div className="card bg-base-100 shadow-sm">
            <div className="card-body p-4 gap-4 flex flex-col h-full">
              <h3 className="font-semibold text-sm text-base-content/60 uppercase tracking-wide">
                Votre commande
              </h3>

              

              {(billingName || billingAddress) && (
                <div className="pb-3 border-b border-base-200">
                  <p className="text-xs text-base-content/40 uppercase tracking-wide mb-1">Facturation</p>
                  {billingName && <p className="text-sm text-base-content/60">{billingName}</p>}
                  {billingAddress && (
                    <p className="text-sm text-base-content/60 whitespace-pre-line">{billingAddress}</p>
                  )}
                </div>
              )}

              <div className="flex flex-col gap-2">
                {selectedServices.map(s => {
                  const qty = lines.find(l => l.service_id === s.id)?.quantity ?? 1
                  return (
                    <div key={s.id} className="flex items-start justify-between gap-2">
                      <span className="text-sm">
                        {qty > 1 && <span className="font-mono text-base-content/60 mr-1">{qty}×</span>}
                        {s.name}
                      </span>
                      <span className="text-sm font-semibold whitespace-nowrap">
                        {(s.price * qty).toFixed(2)} €
                      </span>
                    </div>
                  )
                })}
              </div>

              <div className="flex-1" />

              <div className="pt-3 border-t border-base-200 flex justify-between items-baseline">
                <span className="text-sm text-base-content/50">Total TTC</span>
                <span className="text-2xl font-bold text-primary">{total.toFixed(2)} €</span>
              </div>
            </div>
          </div>

          {/* Right column — payment options */}
          <div className="card bg-base-100 shadow-sm">
            <div className="card-body p-4 gap-4">
              <h3 className="font-semibold text-sm text-base-content/60 uppercase tracking-wide">
                Mode de paiement
              </h3>

              <div className="flex flex-col gap-3">
                {/* Card payment */}
                <div
                  className={`card border-2 transition-all ${
                    !sumup_available
                      ? 'border-base-200 opacity-40 cursor-not-allowed'
                      : paymentMethod === 'card'
                      ? 'border-primary bg-primary/5 cursor-pointer'
                      : 'border-base-200 bg-base-100 hover:border-base-300 cursor-pointer'
                  }`}
                  onClick={() => sumup_available && setPaymentMethod('card')}
                >
                  <div className="card-body p-3 flex-row items-center gap-3">
                    <span className={`w-5 h-5 rounded-full border-2 shrink-0 flex items-center justify-center ${
                      paymentMethod === 'card' && sumup_available
                        ? 'border-primary bg-primary'
                        : 'border-base-300'
                    }`}>
                      {paymentMethod === 'card' && sumup_available && (
                        <span className="w-2 h-2 rounded-full bg-primary-content" />
                      )}
                    </span>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-sm">Carte bancaire</p>
                      <p className="text-xs text-base-content/50">Paiement sécurisé en ligne</p>
                    </div>
                    {!sumup_available && (
                      <span className="badge badge-sm badge-ghost">Non disponible</span>
                    )}
                  </div>
                </div>

                {/* On-site payment */}
                <div
                  className={`card border-2 transition-all cursor-pointer ${
                    paymentMethod === 'on_site'
                      ? 'border-primary bg-primary/5'
                      : 'border-base-200 bg-base-100 hover:border-base-300'
                  }`}
                  onClick={() => setPaymentMethod('on_site')}
                >
                  <div className="card-body p-3 flex-row items-center gap-3">
                    <span className={`w-5 h-5 rounded-full border-2 shrink-0 flex items-center justify-center ${
                      paymentMethod === 'on_site'
                        ? 'border-primary bg-primary'
                        : 'border-base-300'
                    }`}>
                      {paymentMethod === 'on_site' && (
                        <span className="w-2 h-2 rounded-full bg-primary-content" />
                      )}
                    </span>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-sm">Paiement sur place</p>
                      <p className="text-xs text-base-content/50">Règlement auprès d'un coworker</p>
                    </div>
                  </div>
                </div>
              </div>

              <button
                type="button"
                className="btn btn-primary w-full mt-2"
                disabled={submitting}
                onClick={handlePay}
              >
                {submitting
                  ? <span className="loading loading-spinner loading-sm" />
                  : paymentMethod === 'card'
                  ? `Payer ${total.toFixed(2)} €`
                  : 'Confirmer — payer sur place'
                }
              </button>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
