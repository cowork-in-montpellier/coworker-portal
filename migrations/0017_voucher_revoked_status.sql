-- Allow a "Revoked" voucher status (used when a voucher is split into smaller
-- vouchers and the original is cancelled), displayed as "Annulé" in the UI.
ALTER TABLE portal_voucher DROP CONSTRAINT portal_voucher_status_check;
ALTER TABLE portal_voucher ADD CONSTRAINT portal_voucher_status_check
    CHECK (status IN ('Valid', 'Used', 'Expired', 'Unknown', 'Revoked'));
