ALTER TABLE portal_voucher
    ADD CONSTRAINT fk_voucher_bill
        FOREIGN KEY (bill_id)
        REFERENCES billjobs_bill(id)
        ON DELETE CASCADE;

ALTER TABLE portal_voucher
    ADD CONSTRAINT fk_voucher_billline
        FOREIGN KEY (billline_id)
        REFERENCES billjobs_billline(id)
        ON DELETE CASCADE;