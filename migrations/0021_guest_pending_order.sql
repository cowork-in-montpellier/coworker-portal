CREATE TABLE portal_guest_pending_order (
    order_token          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    provisional_number   VARCHAR(20)     NOT NULL UNIQUE,
    payment_method       VARCHAR(20)     NOT NULL DEFAULT 'on_site',
    sumup_checkout_id    VARCHAR(64),
    checkout_failed      BOOLEAN         NOT NULL DEFAULT false,
    guest_email          VARCHAR(254)    NOT NULL,
    billing_address      TEXT            NOT NULL,
    total_amount         DOUBLE PRECISION NOT NULL,
    created_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    expires_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW() + INTERVAL '24 hours',
    paid_at              TIMESTAMPTZ,
    guest_token          UUID
);

CREATE TABLE portal_guest_pending_order_line (
    id           SERIAL  PRIMARY KEY,
    order_token  UUID    NOT NULL REFERENCES portal_guest_pending_order(order_token) ON DELETE CASCADE,
    service_id   INTEGER NOT NULL,
    quantity     INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_guest_pending_order_sumup ON portal_guest_pending_order(sumup_checkout_id)
    WHERE sumup_checkout_id IS NOT NULL;
