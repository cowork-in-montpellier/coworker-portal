import { Document, Page, StyleSheet, Text, View, pdf } from '@react-pdf/renderer'
import type { VoucherStatusEntry } from '../api/bills'

const styles = StyleSheet.create({
  page: {
    padding: 40,
    fontFamily: 'Helvetica',
    backgroundColor: '#f2f2f2',
  },
  title: {
    fontSize: 18,
    fontFamily: 'Helvetica-Bold',
    marginBottom: 4,
  },
  subtitle: {
    fontSize: 10,
    color: '#666',
    marginBottom: 24,
  },
  grid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  card: {
    width: 150,
    backgroundColor: '#ffffff',
    borderRadius: 6,
    border: '1pt solid #d1d5db',
    padding: 12,
  },
  cardExpired: {
    backgroundColor: '#e5e7eb',
    opacity: 0.6,
  },
  cardLabel: {
    fontSize: 8,
    color: '#9ca3af',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 4,
  },
  cardCode: {
    fontSize: 13,
    fontFamily: 'Helvetica-Bold',
    letterSpacing: 1,
    marginBottom: 4,
  },
  cardCodeStruck: {
    fontSize: 13,
    fontFamily: 'Helvetica-Bold',
    letterSpacing: 1,
    marginBottom: 4,
    color: '#9ca3af',
    textDecoration: 'line-through',
  },
  cardDuration: {
    fontSize: 9,
    color: '#6b7280',
    marginBottom: 6,
  },
  badge: {
    alignSelf: 'flex-start',
    borderRadius: 4,
    paddingHorizontal: 5,
    paddingVertical: 2,
  },
  badgeText: {
    fontSize: 8,
    fontFamily: 'Helvetica-Bold',
  },
})

const STATUS_COLORS: Record<string, { bg: string; text: string; label: string }> = {
  Valid:   { bg: '#dcfce7', text: '#166534', label: 'Valide' },
  Used:    { bg: '#f3f4f6', text: '#374151', label: 'Utilisé' },
  Expired: { bg: '#fee2e2', text: '#991b1b', label: 'Expiré' },
  Unknown: { bg: '#f3f4f6', text: '#374151', label: 'Inconnu' },
}

function VoucherCard({ voucher, index }: { voucher: VoucherStatusEntry; index: number }) {
  const isExpired = voucher.status === 'Expired' || voucher.status === 'Used'
  const colors = STATUS_COLORS[voucher.status] ?? STATUS_COLORS.Unknown

  return (
    <View style={[styles.card, isExpired ? styles.cardExpired : {}]}>
      <Text style={styles.cardLabel}>Voucher {index + 1}</Text>
      <Text style={isExpired ? styles.cardCodeStruck : styles.cardCode}>{voucher.code}</Text>
      <Text style={styles.cardDuration}>{voucher.duration}h</Text>
      <View style={[styles.badge, { backgroundColor: colors.bg }]}>
        <Text style={[styles.badgeText, { color: colors.text }]}>{colors.label}</Text>
      </View>
    </View>
  )
}

function VoucherDocument({
  billNumber,
  vouchers,
}: {
  billNumber: string
  vouchers: VoucherStatusEntry[]
}) {
  return (
    <Document>
      <Page size="A4" style={styles.page}>
        <Text style={styles.title}>Vouchers — Facture {billNumber}</Text>
        <Text style={styles.subtitle}>{vouchers.length} voucher{vouchers.length !== 1 ? 's' : ''}</Text>
        <View style={styles.grid}>
          {vouchers.map((v, i) => (
            <VoucherCard key={v.unify_id} voucher={v} index={i} />
          ))}
        </View>
      </Page>
    </Document>
  )
}

export async function generateVoucherPdf(
  billNumber: string,
  vouchers: VoucherStatusEntry[],
): Promise<void> {
  const blob = await pdf(
    <VoucherDocument billNumber={billNumber} vouchers={vouchers} />
  ).toBlob()
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `vouchers-${billNumber}.pdf`
  a.click()
  URL.revokeObjectURL(url)
}
