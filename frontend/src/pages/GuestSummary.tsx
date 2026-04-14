import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import {
  type GuestBillResponse,
  checkGuestVouchers,
  downloadGuestBillPdf,
  getGuestBill,
} from '../api/guest'
import type { VoucherStatusEntry } from '../api/bills'
import { generateVoucherPdf } from '../components/VoucherPdf'

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
        for (const v of b.vouchers) {
          statusMap.set(v.unify_id, v.status)
        }
        setVoucherStatuses(statusMap)
      })
      .catch(() => setError('Impossible de charger la facture.'))
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
      const entries: VoucherStatusEntry[] = bill.vouchers.map(v => ({
        unify_id: v.unify_id,
        code: v.code,
        duration: v.duration,
        status: voucherStatuses.get(v.unify_id) ?? v.status,
      }))
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

  const hasValidVoucher = bill?.vouchers.some(
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
    return (
      <div className="min-h-screen bg-base-200 flex flex-col">
        <div className="navbar bg-base-100 border-b border-base-200 px-4">
          <div className="navbar-start">
            <span className="text-lg font-bold text-primary">Coworking</span>
          </div>
        </div>
        <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
          <div role="alert" className="alert alert-error">
            <span>{error ?? 'Facture introuvable.'}</span>
          </div>
        </main>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-base-200 flex flex-col">
      <div className="navbar bg-base-100 border-b border-base-200 px-4">
        <div className="navbar-start">
          <span className="text-lg font-bold text-primary">Coworking</span>
        </div>
      </div>

      <main className="flex-1 p-4 md:p-8 max-w-2xl mx-auto w-full">
        <div className="mb-6">
          <h2 className="text-xl font-bold">Achat confirmé</h2>
          <p className="text-sm text-base-content/50 mt-1">
            Conservez cette page ou notez vos codes — elle n'est accessible que via ce lien.
          </p>
        </div>

        {/* Bill summary card */}
        <div className="card bg-base-100 shadow-sm mb-6">
          <div className="card-body p-4 gap-2">
            <div className="flex items-start justify-between">
              <div>
                <p className="font-mono font-semibold">{bill.bill_number}</p>
                <p className="text-sm text-base-content/60">{bill.service_name}</p>
                <p className="text-xs text-base-content/40 mt-0.5">{bill.date}</p>
              </div>
              <div className="text-right">
                <p className="text-xl font-bold text-primary">{bill.amount.toFixed(2)} €</p>
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
              </div>
            </div>
          </div>
        </div>

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
              {bill.vouchers.map((v, i) => {
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
