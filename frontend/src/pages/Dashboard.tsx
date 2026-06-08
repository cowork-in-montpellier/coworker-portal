import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { type Bill, type ListBillsResponse, type VoucherStatusEntry, checkVouchers, downloadBillPdf, listBills, revokeVoucher } from '../api/bills'
import { generateVoucherPdf } from '../components/VoucherPdf'
import { type Service, listServices } from '../api/services'
import { Navbar } from '../components/Navbar'
import { useStatus } from '../hooks/useStatus'
import { useToast } from '../lib/toast'
import { ApiError } from '../api/client'

const PAGE_SIZE = 20

function StatusBadge({ isPaid, date }: { isPaid: boolean; date: string }) {
  if (isPaid) return <span className="badge badge-success badge-sm">Paiement confirmé</span>

  const billDate = new Date(date)
  const twoMonthsAgo = new Date()
  twoMonthsAgo.setMonth(twoMonthsAgo.getMonth() - 2)
  if (billDate > twoMonthsAgo) return null

  return <span className="badge badge-warning badge-sm">Retard de paiement ou non validé</span>
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

/** True when the given ISO date (YYYY-MM-DD) falls in a calendar month strictly before the current one. */
function isPastMonth(isoDate: string): boolean {
  return isoDate.slice(0, 7) < new Date().toISOString().slice(0, 7)
}

/** Flatten all vouchers from all lines of a bill into a single list. */
function flattenVouchers(bill: Bill): VoucherStatusEntry[] {
  return bill.lines.flatMap(l => l.vouchers.map(v => ({
    unify_id: v.unify_id,
    code: v.code,
    duration: v.duration,
    status: v.status,
  })))
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
  const [invoiceId, setInvoiceId] = useState<number | null>(null)
  const [copiedVoucherId, setCopiedVoucherId] = useState<string | null>(null)
  const copiedVoucherTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [revokingVoucherId, setRevokingVoucherId] = useState<string | null>(null)
  const toast = useToast()

  const toggleExpand = (id: number) =>
    setExpandedId(prev => (prev === id ? null : id))

  const handleCopyVoucher = (unifyId: string, code: string) => {
    navigator.clipboard.writeText(code.replace(/-/g, '')).then(() => {
      setCopiedVoucherId(unifyId)
      if (copiedVoucherTimer.current) clearTimeout(copiedVoucherTimer.current)
      copiedVoucherTimer.current = setTimeout(() => setCopiedVoucherId(null), 2000)
    })
  }

  const handleRevokeVoucher = async (billId: number, unifyId: string) => {
    if (!window.confirm('Révoquer ce voucher ? Il deviendra immédiatement inutilisable.')) return
    setRevokingVoucherId(unifyId)
    try {
      await revokeVoucher(billId, unifyId)
      await handleCheckVouchers(billId)
      toast('Voucher révoqué', 'success')
    } catch (err) {
      toast(err instanceof ApiError ? err.message : 'Erreur lors de la révocation du voucher', 'error')
    } finally {
      setRevokingVoucherId(null)
    }
  }

  const handleInvoice = async (billId: number, billNumber: string) => {
    setInvoiceId(billId)
    try {
      await downloadBillPdf(billId, billNumber)
    } catch {
      // silently ignore
    } finally {
      setInvoiceId(null)
    }
  }

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
            const allVouchers = flattenVouchers(bill)
            if (allVouchers.length > 0) {
              next.set(bill.id, allVouchers)
            }
          }
          return next
        })
      })
      .catch(e => setError(e instanceof Error ? e.message : 'Impossible de charger les factures'))
      .finally(() => setLoading(false))
  }, [page])

  const { invoice_available } = useStatus()
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
          <div className="flex items-center gap-2">
            <button
              className="btn btn-outline btn-sm"
              onClick={() => navigate('/invite')}
            >
              Inviter un membre
            </button>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => navigate('/bills/new')}
            >
              + Nouvelle facture
            </button>
          </div>
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
                <th>Service(s)</th>
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
                const managedLines = bill.lines.filter(l => l.service_id != null)
                const allUnmanaged = managedLines.length === 0
                const serviceNames = managedLines
                  .map(l => {
                    const name = serviceMap.get(l.service_id!)?.name ?? '—'
                    return l.quantity > 1 ? `${l.quantity}× ${name}` : name
                  })
                  .join(', ')
                const expanded = expandedId === bill.id
                const allVouchers = flattenVouchers(bill)
                const hasVouchers = allVouchers.length > 0

                return (
                  <>
                    <tr
                      key={bill.id}
                      className={allUnmanaged ? 'opacity-50 italic' : 'hover cursor-pointer'}
                      onClick={() => hasVouchers && toggleExpand(bill.id)}
                    >
                      <td className="w-6 text-base-content/30 text-xs">
                        {hasVouchers ? (expanded ? '▾' : '▸') : null}
                      </td>
                      <td className="font-mono">{bill.number}</td>
                      <td>{bill.date}</td>
                      <td className="text-base-content/70">{allUnmanaged ? null : serviceNames}</td>
                      <td className="text-right">{bill.amount.toFixed(2)} €</td>
                      <td>
                        <div className="flex items-center gap-2">
                          {invoice_available && (
                            <button
                              className="btn btn-xs btn-ghost btn-circle"
                              disabled={invoiceId === bill.id}
                              title="Télécharger la facture"
                              onClick={e => { e.stopPropagation(); handleInvoice(bill.id, bill.number) }}
                            >
                              {invoiceId === bill.id
                                ? <span className="loading loading-spinner loading-xs" />
                                : '⎙'}
                            </button>
                          )}
                          {!allUnmanaged && <StatusBadge isPaid={bill.is_paid} date={bill.date} />}
                        </div>
                      </td>
                    </tr>

                    {expanded && hasVouchers && (
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

                          {/* Render vouchers grouped by line; show service name sub-header when multi-line */}
                          <div className="flex flex-col gap-4">
                            {bill.lines.filter(l => l.vouchers.length > 0).map(line => {
                              const lineName = line.service_id != null
                                ? (serviceMap.get(line.service_id)?.name ?? null)
                                : null
                              const lineLabel = lineName
                                ? (line.quantity > 1 ? `${line.quantity}× ${lineName}` : lineName)
                                : null
                              const isMonthly = line.service_id != null
                                && serviceMap.get(line.service_id)?.voucher_spec.kind === 'Monthly'
                              return (
                                <div key={line.id}>
                                  {bill.lines.filter(l => l.vouchers.length > 0).length > 1 && lineLabel && (
                                    <p className="text-xs text-base-content/50 font-medium mb-2">{lineLabel}</p>
                                  )}
                                  <div className="flex flex-wrap gap-3">
                                    {line.vouchers.map((v, i) => {
                                      const liveStatus = voucherStatuses.get(bill.id)?.find(s => s.unify_id === v.unify_id)
                                      const status = liveStatus?.status ?? null
                                      const isExpired = status === 'Expired' || status === 'Used'
                                      const canRevoke = isMonthly && status === 'Valid' && isPastMonth(bill.date)
                                      return (
                                        <div
                                          key={v.unify_id}
                                          className={`card border shadow-sm w-44 transition-opacity group relative ${
                                            isExpired
                                              ? 'bg-base-200 border-base-300 opacity-40'
                                              : 'bg-base-100 border-base-300'
                                          }`}
                                        >
                                          <div className="absolute bottom-1.5 right-1.5 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                                            <button
                                              type="button"
                                              className="btn btn-xs btn-ghost"
                                              title="Copier le code (sans tirets)"
                                              onClick={() => handleCopyVoucher(v.unify_id, v.code)}
                                            >
                                              {copiedVoucherId === v.unify_id ? 'Copié ✓' : 'Copier'}
                                            </button>
                                            {canRevoke && (
                                              <button
                                                type="button"
                                                className="btn btn-xs btn-ghost text-error"
                                                title="Révoquer ce voucher du mois précédent"
                                                disabled={revokingVoucherId === v.unify_id}
                                                onClick={() => handleRevokeVoucher(bill.id, v.unify_id)}
                                              >
                                                {revokingVoucherId === v.unify_id ? '…' : 'Révoquer'}
                                              </button>
                                            )}
                                          </div>
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
                                            {v.active_days_count > 0 && (
                                              <p className="text-xs text-primary/70 font-medium">{v.active_days_count} jour{v.active_days_count > 1 ? 's' : ''} actif{v.active_days_count > 1 ? 's' : ''}</p>
                                            )}
                                          </div>
                                        </div>
                                      )
                                    })}
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
