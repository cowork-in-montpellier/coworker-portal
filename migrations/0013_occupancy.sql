CREATE TABLE portal_occupancy_snapshot (
    id              SERIAL      PRIMARY KEY,
    slot_start      TIMESTAMPTZ NOT NULL,
    connected_count INT         NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX portal_occupancy_snapshot_slot ON portal_occupancy_snapshot(slot_start);
