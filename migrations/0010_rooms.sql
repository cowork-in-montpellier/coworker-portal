CREATE TABLE portal_room (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#3b82f6'
);

CREATE TABLE portal_room_booking (
    id SERIAL PRIMARY KEY,
    room_id INTEGER NOT NULL REFERENCES portal_room(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    start_at TIMESTAMPTZ NOT NULL,
    end_at TIMESTAMPTZ NOT NULL,
    created_by INTEGER NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT booking_end_after_start CHECK (end_at > start_at)
);

CREATE INDEX idx_prb_room_time ON portal_room_booking(room_id, start_at, end_at);
CREATE INDEX idx_prb_time ON portal_room_booking(start_at, end_at);

INSERT INTO portal_room (name, color) VALUES
    ('Titanic', '#3b82f6');
