CREATE TABLE service (
    id                  SERIAL PRIMARY KEY,
    name                VARCHAR(256) NOT NULL,
    description         TEXT NOT NULL,
    price               FLOAT8 NOT NULL,
    kind                VARCHAR(10) NOT NULL CHECK (kind IN ('Monthly', 'Book')),
    amount              INT,        -- null for Monthly; number of vouchers for Book
    duration            INT,        -- null for Monthly; hours per voucher for Book
    external_service_id INT NOT NULL,  -- references billjobs_service.id (external)
    is_available        BOOLEAN NOT NULL DEFAULT true,
    is_guest_available  BOOLEAN NOT NULL DEFAULT false
);