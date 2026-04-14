import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { type ListBillsResponse, type VoucherStatusEntry, checkVouchers, listBills } from '../api/bills'
import { generateVoucherPdf } from '../components/VoucherPdf'
import { type Service, listServices } from '../api/services'
import { Navbar } from '../components/Navbar'

const PAGE_SIZE = 20

function StatusBadge({ isPaid, date }: { isPaid: boolean; date: string }) {
  if (isPaid) return <span className="badge badge-success badge-sm">Payée</span>

  const billDate = new Date(date)
  const twoMonthsAgo = new Date()
  twoMonthsAgo.setMonth(twoMonthsAgo.getMonth() - 2)
  if (billDate > twoMonthsAgo) return null

  return <span className="badge badge-warning badge-sm">Impayée</span>
}

function SkeletonRow() {
  return (
    <tr>
      {Array.from({ length: 6 }).map((_, i) => (
        <td key={i}><div className="skeleton h-4 w-full" /></td>
      ))}
    </tr>
  )
}

export function Dashboard() {
  const navigate = useNavigate()
  const [result, setResult] = useState<ListBillsResponse | null>(null)
  const [serviceMap, setServiceMap] = useState<Map<number, Service>>(new Map())
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [page, setPage] = useState(0)
  const [expandedId, setExpandedId] = useState<number | null>(null)
  const [voucherStatuses, setVoucherStatuses] = useState<Map<number, VoucherStatusEntry[]>>(new Map())
  const [checkingId, setCheckingId] = useState<number | null>(null)
  const [downloadingId, setDownloadingId] = useState<number | null>(null)

  const toggleExpand = (id: number) =>
    setExpandedId(prev => (prev === id ? null : id))

  const handleDownloadPdf = async (billId: number, billNumber: string, vouchers: VoucherStatusEntry[]) => {
    setDownloadingId(billId)
    try {
      await generateVoucherPdf(billNumber, vouchers)
    } catch {
      // silently ignore
    } finally {
      setDownloadingId(null)
    }
  }

  const handleCheckVouchers = async (billId: number) => {
    setCheckingId(billId)
    try {
      const entries = await checkVouchers(billId)
      setVoucherStatuses(prev => new Map(prev).set(billId, entries))
    } catch {
      // silently ignore — stale local data stays visible
    } finally {
      setCheckingId(null)
    }
  }

  useEffect(() => {
    listServices()
      .then(services => setServiceMap(new Map(services.map(s => [s.id, s]))))
      .catch(() => { /* non-fatal: service names just won't show */ })
  }, [])

  useEffect(() => {
    setLoading(true)
    setError(null)
    listBills({ offset: page * PAGE_SIZE, limit: PAGE_SIZE })
      .then(res => {
        setResult(res)
        setVoucherStatuses(prev => {
          const next = new Map(prev)
          for (const bill of res.data) {
            if (bill.kind === 'Managed' && bill.vouchers.length > 0) {
              next.set(bill.id, bill.vouchers.map(v => ({
                unify_id: v.unify_id,
                code: v.code,
                duration: v.duration,
                status: v.status,
              })))
            }
          }
          return next
        })
      })
      .catch(e => setError(e instanceof Error ? e.message : 'Impossible de charger les factures'))
      .finally(() => setLoading(false))
  }, [page])

  const totalPages = result ? Math.ceil(result.total / PAGE_SIZE) : 0

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-6xl mx-auto w-full">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-xl font-bold">Mes factures</h2>
            {result && (
              <p className="text-sm text-base-content/50 mt-0.5">
                {result.total} facture{result.total !== 1 ? 's' : ''} au total
              </p>
            )}
          </div>
          <button
            className="btn btn-primary btn-sm"
            onClick={() => navigate('/bills/new')}
          >
            + Nouvelle facture
          </button>
        </div>

        {error && (
          <div role="alert" className="alert alert-error mb-4">
            <span>{error}</span>
          </div>
        )}

        <div className="card bg-base-100 shadow-sm overflow-x-auto">
          <table className="table table-zebra w-full">
            <thead>
              <tr>
                <th />
                <th>Numéro</th>
                <th>Date</th>
                <th>Service</th>
                <th className="text-right">Montant</th>
                <th>Statut</th>
              </tr>
            </thead>
            <tbody>
              {loading && Array.from({ length: PAGE_SIZE }).map((_, i) => (
                <SkeletonRow key={i} />
              ))}

              {!loading && result?.data.length === 0 && (
                <tr>
                  <td colSpan={6} className="text-center text-base-content/40 py-12">
                    Aucune facture trouvée.
                  </td>
                </tr>
              )}

              {!loading && result?.data.map(bill => {
                const unmanaged = bill.kind === 'Unmanaged'
                const serviceName = bill.kind === 'Managed'
                  ? (serviceMap.get(bill.service_id)?.name ?? '—')
                  : null
                const expanded = expandedId === bill.id
                const hasVouchers = bill.kind === 'Managed' && bill.vouchers.length > 0

                return (
                  <>
                    <tr
                      key={bill.id}
                      className={unmanaged ? 'opacity-50 italic' : 'hover cursor-pointer'}
                      onClick={() => hasVouchers && toggleExpand(bill.id)}
                    >
                      <td className="w-6 text-base-content/30 text-xs">
                        {hasVouchers ? (expanded ? '▾' : '▸') : null}
                      </td>
                      <td className="font-mono">{bill.number}</td>
                      <td>{bill.date}</td>
                      <td className="text-base-content/70">{serviceName}</td>
                      <td className="text-right">{bill.amount.toFixed(2)} €</td>
                      <td>{unmanaged ? null : <StatusBadge isPaid={bill.is_paid} date={bill.date} />}</td>
                    </tr>

                    {expanded && bill.kind === 'Managed' && (
                      <tr key={`${bill.id}-vouchers`} className="bg-base-200/60">
                        <td />
                        <td colSpan={5} className="py-4 px-4">
                          <div className="flex items-center gap-2 mb-3">
                            <span className="text-xs text-base-content/40 font-medium uppercase tracking-wide">Vouchers</span>
                            <button
                              className="btn btn-xs btn-ghost btn-circle"
                              disabled={checkingId === bill.id}
                              onClick={() => handleCheckVouchers(bill.id)}
                              title="Vérifier le statut"
                            >
                              {checkingId === bill.id
                                ? <span className="loading loading-spinner loading-xs" />
                                : '↻'}
                            </button>
                            {(voucherStatuses.get(bill.id) ?? []).some(v => v.status === 'Valid') && (
                              <button
                                className="btn btn-xs btn-ghost btn-circle"
                                disabled={downloadingId === bill.id}
                                onClick={() => handleDownloadPdf(bill.id, bill.number, voucherStatuses.get(bill.id) ?? [])}
                                title="Télécharger le PDF"
                              >
                                {downloadingId === bill.id
                                  ? <span className="loading loading-spinner loading-xs" />
                                  : '⎙'}
                              </button>
                            )}
                          </div>
                          <div className="flex flex-wrap gap-3">
                            {bill.vouchers.map((v, i) => {
                              const liveStatus = voucherStatuses.get(bill.id)?.find(s => s.unify_id === v.unify_id)
                              const status = liveStatus?.status ?? null
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
                                      {status && (
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
                                      )}
                                    </div>
                                    <p className={`font-mono font-semibold text-sm tracking-wide ${isExpired ? 'line-through' : ''}`}>
                                      {v.code}
                                    </p>
                                    <p className="text-xs text-base-content/50">{v.duration}h</p>
                                  </div>
                                </div>
                              )
                            })}
                          </div>
                        </td>
                      </tr>
                    )}
                  </>
                )
              })}
            </tbody>
          </table>
        </div>

        {totalPages > 1 && (
          <div className="flex justify-center mt-6">
            <div className="join">
              <button
                className="join-item btn btn-sm"
                disabled={page === 0}
                onClick={() => setPage(p => p - 1)}
              >
                «
              </button>
              <button className="join-item btn btn-sm pointer-events-none">
                {page + 1} / {totalPages}
              </button>
              <button
                className="join-item btn btn-sm"
                disabled={page >= totalPages - 1}
                onClick={() => setPage(p => p + 1)}
              >
                »
              </button>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
