import { Fragment, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  type Bill,
  type ListBillsResponse,
  type VoucherStatusEntry,
  checkVouchers,
  downloadBillPdf,
  getBill,
  listBills,
  revokeVoucher,
  splitVoucher,
} from '../api/bills'
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
      {Array.from({ length: 5 }).map((_, i) => (
        <td key={i}>
          <div className="skeleton h-4 w-full" />
        </td>
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
  return bill.lines.flatMap(l =>
    l.vouchers.map(v => ({
      unify_id: v.unify_id,
      code: v.code,
      duration: v.duration,
      status: v.status,
    })),
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
  const [invoiceId, setInvoiceId] = useState<number | null>(null)
  const [copiedVoucherId, setCopiedVoucherId] = useState<string | null>(null)
  const copiedVoucherTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [revokingVoucherId, setRevokingVoucherId] = useState<string | null>(null)
  const [splittingVoucherId, setSplittingVoucherId] = useState<string | null>(null)
  const toast = useToast()

  const refreshBill = (updated: Bill) => {
    setResult(prev => (prev ? { ...prev, data: prev.data.map(b => (b.id === updated.id ? updated : b)) } : prev))
  }

  const toggleExpand = (id: number) => setExpandedId(prev => (prev === id ? null : id))

  const handleCopyVoucher = (unifyId: string, code: string) => {
    navigator.clipboard.writeText(code.replace(/-/g, '')).then(() => {
      setCopiedVoucherId(unifyId)
      if (copiedVoucherTimer.current) clearTimeout(copiedVoucherTimer.current)
      copiedVoucherTimer.current = setTimeout(() => setCopiedVoucherId(null), 2000)
    })
  }

  const handleCopyAllVouchers = (billId: number, vouchers: VoucherStatusEntry[]) => {
    const codes = vouchers.map(v => v.code.replace(/-/g, '')).join('\n')
    navigator.clipboard.writeText(codes).then(() => {
      setCopiedVoucherId(`bill-${billId}`)
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

  const handleSplitVoucher = async (billId: number, unifyId: string) => {
    if (!window.confirm('Diviser ce voucher de 10h en 2 vouchers de 5h ? Le voucher original sera annulé.')) return
    setSplittingVoucherId(unifyId)
    try {
      await splitVoucher(billId, unifyId)
      refreshBill(await getBill(billId))
      await handleCheckVouchers(billId)
      toast('Voucher divisé en 2 vouchers de 5h', 'success')
    } catch (err) {
      toast(err instanceof ApiError ? err.message : 'Erreur lors de la division du voucher', 'error')
    } finally {
      setSplittingVoucherId(null)
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
      .catch(() => {
        /* non-fatal: service names just won't show */
      })
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
            <button className="btn btn-outline btn-sm" onClick={() => navigate('/invite')}>
              Inviter un membre
            </button>
            <button className="btn btn-primary btn-sm" onClick={() => navigate('/bills/new')}>
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
                <th>Numéro</th>
                <th>Date</th>
                <th>Service(s)</th>
                <th className="text-right">Montant</th>
                <th>Statut</th>
              </tr>
            </thead>
            <tbody>
              {loading && Array.from({ length: PAGE_SIZE }).map((_, i) => <SkeletonRow key={i} />)}

              {!loading && result?.data.length === 0 && (
                <tr>
                  <td colSpan={5} className="text-center text-base-content/40 py-12">
                    Aucune facture trouvée.
                  </td>
                </tr>
              )}

              {!loading &&
                result?.data.map(bill => {
                  const managedLines = bill.lines.filter(l => l.service_id != null)
                  const allUnmanaged = managedLines.length === 0
                  const expanded = expandedId === bill.id
                  const allVouchers = flattenVouchers(bill)
                  const hasVouchers = allVouchers.length > 0
                  const billStatuses = voucherStatuses.get(bill.id) ?? []
                  const validVoucherCount = allVouchers.filter(v => {
                    const live = billStatuses.find(e => e.unify_id === v.unify_id)
                    return (live?.status ?? v.status) === 'Valid'
                  }).length

                  return (
                    <Fragment key={bill.id}>
                      <tr
                        className={allUnmanaged ? 'opacity-50 italic' : 'hover cursor-pointer'}
                        onClick={() => hasVouchers && toggleExpand(bill.id)}
                      >
                        <td className="font-mono">{bill.number}</td>
                        <td>{bill.date}</td>
                        <td>
                          {hasVouchers ? (
                            <div className="flex items-start gap-1.5">
                              <svg
                                className={`w-3.5 h-3.5 mt-0.5 text-base-content/50 shrink-0 transition-transform duration-200 ${expanded ? 'rotate-90' : ''}`}
                                viewBox="0 0 20 20"
                                fill="currentColor"
                                aria-hidden="true"
                              >
                                <path
                                  fillRule="evenodd"
                                  d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
                                  clipRule="evenodd"
                                />
                              </svg>
                              <ul className="space-y-0.5">
                                {managedLines.map(l => {
                                  const name = serviceMap.get(l.service_id!)?.name ?? '—'
                                  const label = l.quantity > 1 ? `${l.quantity}× ${name}` : name
                                  const validCount = l.vouchers.filter(v => {
                                    const live = billStatuses.find(e => e.unify_id === v.unify_id)
                                    return (live?.status ?? v.status) === 'Valid'
                                  }).length
                                  const totalCount = l.vouchers.length
                                  return (
                                    <li key={l.id} className="flex items-center gap-2">
                                      <span className="text-base-content/70">{label}</span>
                                      {validCount > 0 && (
                                        <span className="badge badge-xs badge-info">
                                          {validCount}/{totalCount} valide{validCount !== 1 ? 's' : ''}
                                        </span>
                                      )}
                                    </li>
                                  )
                                })}
                              </ul>
                            </div>
                          ) : (
                            <span className="text-base-content/70">
                              {allUnmanaged
                                ? null
                                : managedLines
                                    .map(l => {
                                      const name = serviceMap.get(l.service_id!)?.name ?? '—'
                                      return l.quantity > 1 ? `${l.quantity}× ${name}` : name
                                    })
                                    .join(', ')}
                            </span>
                          )}
                        </td>
                        <td className="text-right">{bill.amount.toFixed(2)} €</td>
                        <td>
                          <div className="flex items-center gap-2">
                            {invoice_available && (
                              <button
                                className="btn btn-xs btn-ghost btn-circle"
                                disabled={invoiceId === bill.id}
                                title="Télécharger la facture"
                                onClick={e => {
                                  e.stopPropagation()
                                  handleInvoice(bill.id, bill.number)
                                }}
                              >
                                {invoiceId === bill.id ? <span className="loading loading-spinner loading-xs" /> : '⎙'}
                              </button>
                            )}
                            {!allUnmanaged && <StatusBadge isPaid={bill.is_paid} date={bill.date} />}
                          </div>
                        </td>
                      </tr>

                      {expanded && hasVouchers && (
                        <tr key={`${bill.id}-vouchers`} className="bg-base-200/60">
                          <td colSpan={5} className="py-4 px-4">
                            <div className="flex items-center gap-2 mb-3">
                              <span className="text-xs text-base-content/40 font-medium uppercase tracking-wide">
                                Vouchers ({validVoucherCount}/{allVouchers.length} valides)
                              </span>
                              <button
                                className="btn btn-xs btn-ghost"
                                disabled={checkingId === bill.id}
                                onClick={() => handleCheckVouchers(bill.id)}
                                title="Vérifier le statut de mes vouchers"
                              >
                                {checkingId === bill.id ? <span className="loading loading-spinner loading-xs" /> : '↻'}{' '}
                                Vérifier leur statut
                              </button>
                              {(voucherStatuses.get(bill.id) ?? []).some(v => v.status === 'Valid') && (
                                <button
                                  className="btn btn-xs btn-ghost"
                                  onClick={() => handleCopyAllVouchers(bill.id, voucherStatuses.get(bill.id) ?? [])}
                                  title="Copier tous les codes valides"
                                >
                                  {copiedVoucherId === `bill-${bill.id}` ? '✓ Copié' : '⧉ Copier tous'}
                                </button>
                              )}

                              {(voucherStatuses.get(bill.id) ?? []).some(v => v.status === 'Valid') && (
                                <button
                                  className="btn btn-xs btn-ghost"
                                  disabled={downloadingId === bill.id}
                                  onClick={() =>
                                    handleDownloadPdf(bill.id, bill.number, voucherStatuses.get(bill.id) ?? [])
                                  }
                                  title="Télécharger le PDF"
                                >
                                  {downloadingId === bill.id ? (
                                    <span className="loading loading-spinner loading-xs" />
                                  ) : (
                                    '⎙'
                                  )}{' '}
                                  Télécharger au format PDF
                                </button>
                              )}
                            </div>

                            {/* Render vouchers grouped by line; show service name sub-header when multi-line */}
                            <div className="flex flex-col gap-4">
                              {bill.lines
                                .filter(l => l.vouchers.length > 0)
                                .map(line => {
                                  const lineName =
                                    line.service_id != null ? (serviceMap.get(line.service_id)?.name ?? null) : null
                                  const lineLabel = lineName
                                    ? line.quantity > 1
                                      ? `${line.quantity}× ${lineName}`
                                      : lineName
                                    : null
                                  const isMonthly =
                                    line.service_id != null &&
                                    serviceMap.get(line.service_id)?.voucher_spec.kind === 'Monthly'
                                  return (
                                    <div key={line.id}>
                                      {bill.lines.filter(l => l.vouchers.length > 0).length > 1 && lineLabel && (
                                        <p className="text-xs text-base-content/50 font-medium mb-2">{lineLabel}</p>
                                      )}
                                      <div className="flex flex-wrap gap-3">
                                        {[...line.vouchers]
                                          .sort(
                                            (a, b) =>
                                              a.unify_create_time - b.unify_create_time ||
                                              a.unify_id.localeCompare(b.unify_id),
                                          )
                                          .map((v, i) => {
                                            const liveStatus = voucherStatuses
                                              .get(bill.id)
                                              ?.find(s => s.unify_id === v.unify_id)
                                            const status = liveStatus?.status ?? null
                                            const isExpired =
                                              status === 'Expired' || status === 'Used' || status === 'Revoked'
                                            const canRevoke = isMonthly && isPastMonth(bill.date)
                                            const canSplit = v.duration === 10 && status === 'Valid'
                                            return (
                                              <div
                                                key={v.unify_id}
                                                className={`card border shadow-sm w-44 transition-opacity group relative ${
                                                  isExpired
                                                    ? 'bg-base-200 border-base-300 opacity-40'
                                                    : 'bg-base-100 border-base-300'
                                                }`}
                                              >
                                                <div className="absolute bottom-1.5 right-1.5 flex gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity z-10">
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
                                                  {canSplit && (
                                                    <div className="dropdown dropdown-end dropdown-top">
                                                      <div
                                                        tabIndex={0}
                                                        role="button"
                                                        className="btn btn-xs btn-ghost btn-circle"
                                                        title="Options du voucher"
                                                      >
                                                        <svg
                                                          xmlns="http://www.w3.org/2000/svg"
                                                          className="h-3.5 w-3.5"
                                                          viewBox="0 0 20 20"
                                                          fill="currentColor"
                                                        >
                                                          <path
                                                            fillRule="evenodd"
                                                            d="M8.34 1.804A1 1 0 019.32 1h1.36a1 1 0 01.98.804l.214 1.07a6.98 6.98 0 011.66.958l1.036-.336a1 1 0 011.208.502l.68 1.178a1 1 0 01-.223 1.263l-.837.717a7.014 7.014 0 010 1.916l.837.717a1 1 0 01.223 1.263l-.68 1.178a1 1 0 01-1.208.502l-1.035-.336a6.976 6.976 0 01-1.66.958l-.214 1.07a1 1 0 01-.98.804H9.32a1 1 0 01-.98-.804l-.214-1.07a6.98 6.98 0 01-1.66-.958l-1.036.336a1 1 0 01-1.208-.502l-.68-1.178a1 1 0 01.223-1.263l.837-.717a7.014 7.014 0 010-1.916l-.837-.717a1 1 0 01-.223-1.263l.68-1.178a1 1 0 011.208-.502l1.035.336a6.976 6.976 0 011.66-.958l.214-1.07zM10 13a3 3 0 100-6 3 3 0 000 6z"
                                                            clipRule="evenodd"
                                                          />
                                                        </svg>
                                                      </div>
                                                      <ul
                                                        tabIndex={0}
                                                        className="dropdown-content menu bg-base-100 rounded-box shadow-lg border border-base-200 z-20 w-48 p-1"
                                                      >
                                                        <li>
                                                          <button
                                                            type="button"
                                                            disabled={splittingVoucherId === v.unify_id}
                                                            onClick={() => handleSplitVoucher(bill.id, v.unify_id)}
                                                          >
                                                            {splittingVoucherId === v.unify_id
                                                              ? '…'
                                                              : 'Diviser en 2 × 5h'}
                                                          </button>
                                                        </li>
                                                      </ul>
                                                    </div>
                                                  )}
                                                </div>
                                                <div className="card-body p-3 gap-1">
                                                  <div className="flex items-center justify-between">
                                                    <p className="text-xs text-base-content/40 font-medium">
                                                      Voucher {i + 1}
                                                    </p>
                                                    {status && (
                                                      <span
                                                        className={`badge badge-xs ${
                                                          status === 'Valid'
                                                            ? 'badge-success'
                                                            : status === 'Used'
                                                              ? 'badge-neutral'
                                                              : status === 'Expired'
                                                                ? 'badge-error'
                                                                : status === 'Revoked'
                                                                  ? 'badge-warning'
                                                                  : 'badge-ghost'
                                                        }`}
                                                      >
                                                        {status === 'Valid'
                                                          ? 'Valide'
                                                          : status === 'Used'
                                                            ? 'Utilisé'
                                                            : status === 'Expired'
                                                              ? 'Expiré'
                                                              : status === 'Revoked'
                                                                ? 'Annulé'
                                                                : 'Inconnu'}
                                                      </span>
                                                    )}
                                                  </div>
                                                  <p
                                                    className={`font-mono font-semibold text-sm tracking-wide ${isExpired ? 'line-through' : ''}`}
                                                  >
                                                    {v.code}
                                                  </p>
                                                  <p className="text-xs text-base-content/50">{v.duration}h</p>
                                                  {v.active_days_count > 0 && (
                                                    <p className="text-xs text-primary/70 font-medium">
                                                      {v.active_days_count} jour{v.active_days_count > 1 ? 's' : ''}{' '}
                                                      actif
                                                      {v.active_days_count > 1 ? 's' : ''}
                                                    </p>
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
                    </Fragment>
                  )
                })}
            </tbody>
          </table>
        </div>

        {totalPages > 1 && (
          <div className="flex justify-center mt-6">
            <div className="join">
              <button className="join-item btn btn-sm" disabled={page === 0} onClick={() => setPage(p => p - 1)}>
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
