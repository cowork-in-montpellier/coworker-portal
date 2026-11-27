import { format, parseISO } from 'date-fns'
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
import { PageHeader } from '../components/PageHeader'
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

/** `2026-11-25` → `25/11/2026`. */
function formatDate(isoDate: string): string {
  return format(parseISO(isoDate), 'dd/MM/yyyy')
}

function Chevron({ open, className = '' }: { open: boolean; className?: string }) {
  return (
    <svg
      className={`w-3.5 h-3.5 text-base-content/50 shrink-0 transition-transform duration-200 ${open ? 'rotate-90' : ''} ${className}`}
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
  )
}

function SkeletonCard() {
  return (
    <li className="card cw-card p-4 flex flex-col gap-3">
      <div className="flex justify-between gap-4">
        <div className="skeleton h-4 w-28" />
        <div className="skeleton h-4 w-16" />
      </div>
      <div className="skeleton h-3 w-20" />
      <div className="skeleton h-3 w-40" />
    </li>
  )
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
  return bill.lines.flatMap((l) =>
    l.vouchers.map((v) => ({
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
    setResult((prev) => (prev ? { ...prev, data: prev.data.map((b) => (b.id === updated.id ? updated : b)) } : prev))
  }

  const toggleExpand = (id: number) => setExpandedId((prev) => (prev === id ? null : id))

  const handleCopyVoucher = (unifyId: string, code: string) => {
    navigator.clipboard.writeText(code.replace(/-/g, '')).then(() => {
      setCopiedVoucherId(unifyId)
      if (copiedVoucherTimer.current) clearTimeout(copiedVoucherTimer.current)
      copiedVoucherTimer.current = setTimeout(() => setCopiedVoucherId(null), 2000)
    })
  }

  const handleCopyAllVouchers = (billId: number, vouchers: VoucherStatusEntry[]) => {
    const codes = vouchers.map((v) => v.code.replace(/-/g, '')).join('\n')
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
      setVoucherStatuses((prev) => new Map(prev).set(billId, entries))
    } catch {
      // silently ignore — stale local data stays visible
    } finally {
      setCheckingId(null)
    }
  }

  useEffect(() => {
    listServices()
      .then((services) => setServiceMap(new Map(services.map((s) => [s.id, s]))))
      .catch(() => {
        /* non-fatal: service names just won't show */
      })
  }, [])

  useEffect(() => {
    setLoading(true)
    setError(null)
    listBills({ offset: page * PAGE_SIZE, limit: PAGE_SIZE })
      .then((res) => {
        setResult(res)
        setVoucherStatuses((prev) => {
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
      .catch((e) => setError(e instanceof Error ? e.message : 'Impossible de charger les factures'))
      .finally(() => setLoading(false))
  }, [page])

  const { invoice_available } = useStatus()
  const totalPages = result ? Math.ceil(result.total / PAGE_SIZE) : 0

  // Everything the table row and the mobile card both need to show a bill.
  const billView = (bill: Bill) => {
    const managedLines = bill.lines.filter((l) => l.service_id != null)
    const allVouchers = flattenVouchers(bill)
    const billStatuses = voucherStatuses.get(bill.id) ?? []
    const isValid = (v: { unify_id: string; status: VoucherStatusEntry['status'] }) =>
      (billStatuses.find((e) => e.unify_id === v.unify_id)?.status ?? v.status) === 'Valid'
    return {
      managedLines,
      allUnmanaged: managedLines.length === 0,
      expanded: expandedId === bill.id,
      allVouchers,
      hasVouchers: allVouchers.length > 0,
      billStatuses,
      isValid,
      validVoucherCount: allVouchers.filter(isValid).length,
    }
  }

  const lineName = (serviceId: number, quantity: number) => {
    const name = serviceMap.get(serviceId)?.name ?? '—'
    return quantity > 1 ? `${quantity}× ${name}` : name
  }

  const renderServices = (bill: Bill) => {
    const { managedLines, allUnmanaged, hasVouchers, isValid } = billView(bill)
    if (!hasVouchers) {
      return (
        <span className="text-base-content/70">
          {allUnmanaged ? null : managedLines.map((l) => lineName(l.service_id!, l.quantity)).join(', ')}
        </span>
      )
    }
    return (
      <ul className="space-y-1">
        {managedLines.map((l) => {
          const validCount = l.vouchers.filter(isValid).length
          return (
            <li key={l.id} className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
              <span className="text-base-content/70">{lineName(l.service_id!, l.quantity)}</span>
              {validCount > 0 && (
                <span className="badge badge-xs badge-info whitespace-nowrap">
                  {validCount}/{l.vouchers.length} valide{validCount !== 1 ? 's' : ''}
                </span>
              )}
            </li>
          )
        })}
      </ul>
    )
  }

  const renderInvoiceButton = (bill: Bill) =>
    invoice_available && (
      <button
        className="btn btn-xs btn-ghost btn-circle"
        disabled={invoiceId === bill.id}
        title="Télécharger la facture"
        aria-label={`Télécharger la facture ${bill.number}`}
        onClick={(e) => {
          e.stopPropagation()
          handleInvoice(bill.id, bill.number)
        }}
      >
        {invoiceId === bill.id ? <span className="loading loading-spinner loading-xs" /> : '⎙'}
      </button>
    )

  const renderVoucherPanel = (bill: Bill) => {
    const { allVouchers, validVoucherCount } = billView(bill)
    const liveStatuses = voucherStatuses.get(bill.id) ?? []
    const anyValid = liveStatuses.some((v) => v.status === 'Valid')
    const linesWithVouchers = bill.lines.filter((l) => l.vouchers.length > 0)
    return (
      <>
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 mb-3">
          <span className="text-xs text-base-content/40 font-medium uppercase tracking-wide w-full sm:w-auto">
            Vouchers ({validVoucherCount}/{allVouchers.length} valides)
          </span>
          <button
            className="btn btn-xs btn-ghost"
            disabled={checkingId === bill.id}
            onClick={() => handleCheckVouchers(bill.id)}
            title="Vérifier le statut de mes vouchers"
          >
            {checkingId === bill.id ? <span className="loading loading-spinner loading-xs" /> : '↻'} Vérifier leur
            statut
          </button>
          {anyValid && (
            <button
              className="btn btn-xs btn-ghost"
              onClick={() => handleCopyAllVouchers(bill.id, liveStatuses)}
              title="Copier tous les codes valides"
            >
              {copiedVoucherId === `bill-${bill.id}` ? '✓ Copié' : '⧉ Copier tous'}
            </button>
          )}
          {anyValid && (
            <button
              className="btn btn-xs btn-ghost"
              disabled={downloadingId === bill.id}
              onClick={() => handleDownloadPdf(bill.id, bill.number, liveStatuses)}
              title="Télécharger le PDF"
            >
              {downloadingId === bill.id ? <span className="loading loading-spinner loading-xs" /> : '⎙'} Télécharger au
              format PDF
            </button>
          )}
        </div>

        {/* Vouchers grouped by line, with the service name as a sub-header when there are several lines */}
        <div className="flex flex-col gap-4">
          {linesWithVouchers.map((line) => {
            const lineLabel = line.service_id != null ? lineName(line.service_id, line.quantity) : null
            const isMonthly =
              line.service_id != null && serviceMap.get(line.service_id)?.voucher_spec.kind === 'Monthly'
            return (
              <div key={line.id}>
                {linesWithVouchers.length > 1 && lineLabel && (
                  <p className="text-xs text-base-content/50 font-medium mb-2">{lineLabel}</p>
                )}
                <div className="grid grid-cols-2 gap-2 md:flex md:flex-wrap md:gap-3">
                  {[...line.vouchers]
                    .sort((a, b) => a.unify_create_time - b.unify_create_time || a.unify_id.localeCompare(b.unify_id))
                    .map((v, i) => {
                      const status = liveStatuses.find((s) => s.unify_id === v.unify_id)?.status ?? null
                      const isExpired = status === 'Expired' || status === 'Used' || status === 'Revoked'
                      const canRevoke = isMonthly && isPastMonth(bill.date)
                      const canSplit = v.duration === 10 && status === 'Valid'
                      return (
                        <div
                          key={v.unify_id}
                          className={`card border-2 min-w-0 md:w-44 transition-opacity group relative ${
                            isExpired ? 'bg-base-200 border-base-300 opacity-40' : 'bg-base-100 border-base-300'
                          }`}
                        >
                          <div className="card-body p-3 gap-1">
                            <div className="flex items-center justify-between gap-1">
                              <p className="text-xs text-base-content/40 font-medium">Voucher {i + 1}</p>
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
                              className={`font-mono font-semibold text-sm tracking-wide break-all ${isExpired ? 'line-through' : ''}`}
                            >
                              {v.code}
                            </p>
                            <p className="text-xs text-base-content/50">{v.duration}h</p>
                            {v.active_days_count > 0 && (
                              <p className="text-xs text-primary/70 font-medium">
                                {v.active_days_count} jour{v.active_days_count > 1 ? 's' : ''} actif
                                {v.active_days_count > 1 ? 's' : ''}
                              </p>
                            )}
                          </div>
                          {/* Always shown on touch screens; revealed on hover from md up */}
                          <div className="flex justify-end gap-1 px-1.5 pb-1.5 -mt-2 md:mt-0 md:absolute md:bottom-1.5 md:right-1.5 md:p-0 md:opacity-0 md:group-hover:opacity-100 md:focus-within:opacity-100 transition-opacity z-10">
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
                                  aria-label="Options du voucher"
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
                                      {splittingVoucherId === v.unify_id ? '…' : 'Diviser en 2 × 5h'}
                                    </button>
                                  </li>
                                </ul>
                              </div>
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
      </>
    )
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <Navbar />

      <main className="flex-1 p-4 md:p-8 max-w-6xl mx-auto w-full">
        <PageHeader
          title="Mes factures"
          lede={result && `${result.total} facture${result.total !== 1 ? 's' : ''} au total`}
          actions={
            <>
              <button className="btn btn-outline btn-sm flex-1 sm:flex-none" onClick={() => navigate('/invite')}>
                Inviter un membre
              </button>
              <button className="btn btn-primary btn-sm flex-1 sm:flex-none" onClick={() => navigate('/bills/new')}>
                + Nouvelle facture
              </button>
            </>
          }
          className="mb-8"
        />

        {error && (
          <div role="alert" className="alert alert-error mb-4">
            <span>{error}</span>
          </div>
        )}

        {/* Phones: one card per bill */}
        <ul className="flex flex-col gap-3 md:hidden">
          {loading && Array.from({ length: 6 }).map((_, i) => <SkeletonCard key={i} />)}

          {!loading && result?.data.length === 0 && (
            <li className="card cw-card text-center text-base-content/40 py-12">Aucune facture trouvée.</li>
          )}

          {!loading &&
            result?.data.map((bill) => {
              const { allUnmanaged, expanded, hasVouchers, allVouchers, validVoucherCount } = billView(bill)
              return (
                <li key={bill.id} className={`card cw-card ${allUnmanaged ? 'opacity-50 italic' : ''}`}>
                  <div className="p-4 flex flex-col gap-2">
                    <div className="flex items-baseline justify-between gap-3">
                      <span className="font-mono text-sm">{bill.number}</span>
                      <span className="font-semibold whitespace-nowrap">{bill.amount.toFixed(2)} €</span>
                    </div>
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <span className="text-sm text-base-content/60">{formatDate(bill.date)}</span>
                      <div className="flex items-center gap-1">
                        {!allUnmanaged && <StatusBadge isPaid={bill.is_paid} date={bill.date} />}
                        {renderInvoiceButton(bill)}
                      </div>
                    </div>
                    {!allUnmanaged && <div className="text-sm">{renderServices(bill)}</div>}
                  </div>
                  {hasVouchers && (
                    <>
                      <button
                        type="button"
                        className="flex items-center justify-between gap-2 border-t border-base-200 px-4 py-3 text-sm font-medium"
                        aria-expanded={expanded}
                        onClick={() => toggleExpand(bill.id)}
                      >
                        <span>
                          {expanded ? 'Masquer les vouchers' : 'Voir les vouchers'}
                          <span className="text-base-content/50 font-normal">
                            {' '}
                            · {validVoucherCount}/{allVouchers.length} valides
                          </span>
                        </span>
                        <Chevron open={expanded} />
                      </button>
                      {expanded && (
                        <div className="border-t border-base-200 bg-base-200/60 p-3 rounded-b-box">
                          {renderVoucherPanel(bill)}
                        </div>
                      )}
                    </>
                  )}
                </li>
              )
            })}
        </ul>

        {/* From md up: the table */}
        <div className="card cw-card overflow-x-auto hidden md:block">
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
                result?.data.map((bill) => {
                  const { allUnmanaged, expanded, hasVouchers } = billView(bill)
                  return (
                    <Fragment key={bill.id}>
                      <tr
                        className={allUnmanaged ? 'opacity-50 italic' : 'hover cursor-pointer'}
                        onClick={() => hasVouchers && toggleExpand(bill.id)}
                      >
                        <td className="font-mono whitespace-nowrap">{bill.number}</td>
                        <td className="whitespace-nowrap">{formatDate(bill.date)}</td>
                        <td>
                          {hasVouchers ? (
                            <div className="flex items-start gap-1.5">
                              <Chevron open={expanded} className="mt-0.5" />
                              {renderServices(bill)}
                            </div>
                          ) : (
                            renderServices(bill)
                          )}
                        </td>
                        <td className="text-right whitespace-nowrap">{bill.amount.toFixed(2)} €</td>
                        <td>
                          <div className="flex items-center gap-2">
                            {renderInvoiceButton(bill)}
                            {!allUnmanaged && <StatusBadge isPaid={bill.is_paid} date={bill.date} />}
                          </div>
                        </td>
                      </tr>

                      {expanded && hasVouchers && (
                        <tr className="bg-base-200/60">
                          <td colSpan={5} className="py-4 px-4">
                            {renderVoucherPanel(bill)}
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
              <button className="join-item btn btn-sm" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
                «
              </button>
              <button className="join-item btn btn-sm pointer-events-none">
                {page + 1} / {totalPages}
              </button>
              <button
                className="join-item btn btn-sm"
                disabled={page >= totalPages - 1}
                onClick={() => setPage((p) => p + 1)}
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
