import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { type Service, type VoucherSpec } from '../api/services'
import { listGuestServices, createGuestBill } from '../api/guest'

function voucherSpecLabel(spec: VoucherSpec): string {
  if (spec.kind === 'Monthly') return `1 voucher — valable jusqu'à fin de mois`
  return `${spec.amount} voucher${spec.amount > 1 ? 's' : ''} × ${spec.duration}h`
}

function ServiceCard({
  service,
  selected,
  onSelect,
}: {
  service: Service
  selected: boolean
  onSelect: () => void
}) {
  return (
    <label className="cursor-pointer">
      <input
        type="radio"
        name="service"
        className="sr-only"
        checked={selected}
        onChange={onSelect}
      />
      <div
        className={`card border-2 transition-all ${
          selected
            ? 'border-primary bg-primary/5'
            : 'border-base-200 bg-base-100 hover:border-base-300'
        }`}
      >
        <div className="card-body p-4 gap-1">
          <div className="flex items-start justify-between gap-2">
            <h3 className="font-semibold text-base">{service.name}</h3>
            <span className="text-lg font-bold text-primary whitespace-nowrap">
              {service.price.toFixed(2)} €
            </span>
          </div>
          <p className="text-sm text-base-content/60">{service.description}</p>
          <p className="text-xs text-base-content/40 mt-1">
            {voucherSpecLabel(service.voucher_spec)}
          </p>
        </div>
      </div>
    </label>
  )
}

export function GuestBuy() {
  const navigate = useNavigate()
  const [services, setServices] = useState<Service[]>([])
  const [loadingServices, setLoadingServices] = useState(true)
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [billingName, setBillingName] = useState('')
  const [billingAddress, setBillingAddress] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listGuestServices()
      .then(s => {
        setServices(s)
        if (s.length > 0) setSelectedId(s[0].id)
      })
      .catch(() => setError('Impossible de charger les services.'))
      .finally(() => setLoadingServices(false))
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (selectedId === null) return

    setSubmitting(true)
    setError(null)
    try {
      const result = await createGuestBill({
        service_id: selectedId,
        billing_name: billingName || undefined,
        billing_address: billingAddress || undefined,
      })
      navigate(`/buy/summary/${result.guest_token}`)
    } catch {
      setError('Échec de la création de la facture. Veuillez réessayer.')
    } finally {
      setSubmitting(false)
    }
  }

  const selected = services.find(s => s.id === selectedId) ?? null

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <div className="navbar bg-base-100 border-b border-base-200 px-4">
        <div className="navbar-start">
          <span className="text-lg font-bold text-primary">Coworking</span>
        </div>
      </div>

      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <div className="mb-6">
          <h2 className="text-xl font-bold">Acheter un accès</h2>
          <p className="text-sm text-base-content/50 mt-1">
            Aucun compte nécessaire — vos vouchers seront affichés immédiatement après l'achat.
          </p>
        </div>

        {error && (
          <div role="alert" className="alert alert-error mb-4">
            <span>{error}</span>
          </div>
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
            <div className="flex flex-col gap-3">
              {services.map(service => (
                <ServiceCard
                  key={service.id}
                  service={service}
                  selected={selectedId === service.id}
                  onSelect={() => setSelectedId(service.id)}
                />
              ))}
            </div>

            <div className="card bg-base-100 shadow-sm">
              <div className="card-body p-4 gap-4">
                <h3 className="font-semibold text-sm text-base-content/60 uppercase tracking-wide">
                  Facturation (optionnel)
                </h3>
                <div className="form-control gap-1">
                  <label className="label py-0">
                    <span className="label-text text-sm">Nom</span>
                  </label>
                  <input
                    type="text"
                    className="input input-bordered input-sm"
                    placeholder="Prénom Nom"
                    value={billingName}
                    onChange={e => setBillingName(e.target.value)}
                  />
                </div>
                <div className="form-control gap-1">
                  <label className="label py-0">
                    <span className="label-text text-sm">Adresse</span>
                  </label>
                  <textarea
                    className="textarea textarea-bordered textarea-sm"
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
                    {selected ? `${selected.price.toFixed(2)} €` : '—'}
                  </p>
                </div>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={selectedId === null || submitting}
                >
                  {submitting
                    ? <span className="loading loading-spinner loading-sm" />
                    : 'Confirmer & acheter'
                  }
                </button>
              </div>
            </div>
          </form>
        )}
      </main>
    </div>
  )
}
