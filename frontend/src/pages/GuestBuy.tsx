import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { type Service, type VoucherSpec } from '../api/services'
import { listGuestServices } from '../api/guest'
import { Navbar } from '../components/Navbar'

function voucherSpecLabel(spec: VoucherSpec): string {
  if (spec.kind === 'Monthly') return `1 voucher — valable 30 jours`
  return `${spec.amount} voucher${spec.amount > 1 ? 's' : ''} × ${spec.duration}h`
}

function ServiceCard({
  service,
  quantity,
  allowMultiple,
  onToggle,
  onChangeQty,
}: {
  service: Service
  quantity: number   // 0 = not selected
  allowMultiple: boolean
  onToggle: () => void
  onChangeQty: (q: number) => void
}) {
  const selected = quantity > 0
  return (
    <div
      className={`card border-2 transition-all cursor-pointer ${
        selected ? 'border-primary bg-primary/5' : 'border-base-200 bg-base-100 hover:border-base-300'
      }`}
      onClick={onToggle}
    >
      <div className="card-body p-4 gap-1">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <span className={`w-5 h-5 rounded border-2 shrink-0 flex items-center justify-center text-xs font-bold transition-colors ${
              selected ? 'border-primary bg-primary text-primary-content' : 'border-base-300'
            }`}>
              {selected && '✓'}
            </span>
            <h3 className="font-semibold text-base truncate">{service.name}</h3>
          </div>
          <span className="text-lg font-bold text-primary whitespace-nowrap shrink-0">
            {service.price.toFixed(2)} €
          </span>
        </div>
        <p className="text-sm text-base-content/60 pl-7">{service.description}</p>
        <p className="text-xs text-base-content/40 mt-0.5 pl-7">{voucherSpecLabel(service.voucher_spec)}</p>

        {selected && allowMultiple && (
          <div
            className="flex items-center justify-end gap-2 mt-2 pt-2 border-t border-base-200"
            onClick={e => e.stopPropagation()}
          >
            <span className="text-xs text-base-content/50 mr-auto pl-7">Quantité</span>
            <button
              type="button"
              className="btn btn-xs btn-ghost btn-circle"
              onClick={() => onChangeQty(quantity - 1)}
            >
              −
            </button>
            <span className="w-8 text-center font-mono font-semibold text-sm">{quantity}</span>
            <button
              type="button"
              className="btn btn-xs btn-ghost btn-circle"
              onClick={() => onChangeQty(quantity + 1)}
            >
              +
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

export function GuestBuy() {
  const navigate = useNavigate()
  const [services, setServices] = useState<Service[]>([])
  const [loadingServices, setLoadingServices] = useState(true)
  // Map: service_id → quantity (absent = not selected)
  const [quantities, setQuantities] = useState<Map<number, number>>(new Map())
  const [billingName, setBillingName] = useState('')
  const [billingAddress, setBillingAddress] = useState('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listGuestServices()
      .then(s => setServices(s))
      .catch(() => setError('Impossible de charger les services.'))
      .finally(() => setLoadingServices(false))
  }, [])

  const toggleService = (id: number) => {
    setQuantities(prev => {
      const next = new Map(prev)
      if (next.has(id)) next.delete(id)
      else next.set(id, 1)
      return next
    })
  }

  const changeQty = (id: number, qty: number) => {
    setQuantities(prev => {
      const next = new Map(prev)
      if (qty <= 0) next.delete(id)
      else next.set(id, qty)
      return next
    })
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (quantities.size === 0) return
    setError(null)
    const lines = Array.from(quantities.entries()).map(([service_id, quantity]) => ({ service_id, quantity }))
    navigate('/buy/checkout', {
      state: {
        lines,
        selectedServices,
        billingName: billingName || undefined,
        billingAddress: billingAddress || undefined,
        total,
      },
    })
  }

  const allowsMultiple = (s: Service) =>
    s.voucher_spec.kind === 'Book' && s.voucher_spec.amount > 0
  const singleServices = services.filter(s => !allowsMultiple(s))
  const multiServices = services.filter(s => allowsMultiple(s))
  const selectedServices = services.filter(s => quantities.has(s.id))
  const total = selectedServices.reduce((sum, s) => sum + s.price * (quantities.get(s.id) ?? 1), 0)
  const hasBothColumns = singleServices.length > 0 && multiServices.length > 0

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-4xl mx-auto w-full">
        <div className="mb-6">
          <h2 className="text-xl font-bold">Acheter un accès</h2>
          <p className="text-sm text-base-content/50 mt-1">
            Aucun compte nécessaire — vos vouchers seront affichés immédiatement après l'achat.
          </p>
        </div>

        {error && (
          <div role="alert" className="alert alert-error mb-4"><span>{error}</span></div>
        )}

        {loadingServices ? (
          <div className="flex flex-col gap-3">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="skeleton h-24 w-full rounded-2xl" />
            ))}
          </div>
        ) : services.length === 0 ? (
          <div className="text-center text-base-content/40 py-12">
            Aucun service disponible pour l'achat sans compte.
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="flex flex-col gap-6">
            <div className={`grid gap-6 ${hasBothColumns ? 'md:grid-cols-2' : 'grid-cols-1'}`}>
              {singleServices.length > 0 && (
                <div className="flex flex-col gap-3">
                  <p className="text-xs font-medium text-base-content/40 uppercase tracking-wide">
                    Accès mensuel
                  </p>
                  {singleServices.map(service => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      quantity={quantities.get(service.id) ?? 0}
                      allowMultiple={false}
                      onToggle={() => toggleService(service.id)}
                      onChangeQty={q => changeQty(service.id, q)}
                    />
                  ))}
                </div>
              )}
              {multiServices.length > 0 && (
                <div className="flex flex-col gap-3">
                  <p className="text-xs font-medium text-base-content/40 uppercase tracking-wide">
                    Carnets de vouchers
                  </p>
                  {multiServices.map(service => (
                    <ServiceCard
                      key={service.id}
                      service={service}
                      quantity={quantities.get(service.id) ?? 0}
                      allowMultiple={allowsMultiple(service)}
                      onToggle={() => toggleService(service.id)}
                      onChangeQty={q => changeQty(service.id, q)}
                    />
                  ))}
                </div>
              )}
            </div>

            <div className="card bg-base-100 shadow-sm">
              <div className="card-body p-4 gap-3">
                <h3 className="font-semibold text-sm text-base-content/60 uppercase tracking-wide">
                  Facturation (optionnel)
                </h3>
                <div className="flex items-center gap-3">
                  <label className="text-sm w-20 shrink-0 text-right text-base-content/60">Nom</label>
                  <input
                    type="text"
                    className="input input-bordered input-sm flex-1"
                    placeholder="Prénom Nom"
                    value={billingName}
                    onChange={e => setBillingName(e.target.value)}
                  />
                </div>
                <div className="flex items-start gap-3">
                  <label className="text-sm w-20 shrink-0 text-right text-base-content/60 pt-1.5">Adresse</label>
                  <textarea
                    className="textarea textarea-bordered textarea-sm flex-1"
                    placeholder={"12 rue de la Paix\n75001 Paris"}
                    rows={3}
                    value={billingAddress}
                    onChange={e => setBillingAddress(e.target.value)}
                  />
                </div>
              </div>
            </div>

            <div className="card bg-base-100 shadow-sm">
              <div className="card-body p-4 flex-row items-center justify-between">
                <div>
                  <p className="text-sm text-base-content/50">Total TTC</p>
                  <p className="text-2xl font-bold text-primary">
                    {quantities.size > 0 ? `${total.toFixed(2)} €` : '—'}
                  </p>
                  {selectedServices.length > 0 && (
                    <p className="text-xs text-base-content/40 mt-0.5">
                      {selectedServices.map(s => {
                        const q = quantities.get(s.id) ?? 1
                        return q > 1 ? `${q}× ${s.name}` : s.name
                      }).join(' + ')}
                    </p>
                  )}
                </div>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={quantities.size === 0}
                >
                  Confirmer
                </button>
              </div>
            </div>
          </form>
        )}
      </main>
    </div>
  )
}
