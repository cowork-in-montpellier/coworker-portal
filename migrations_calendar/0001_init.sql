-- The calendar's own database, kept apart from the billing one. Bookings refer to their
-- author by `auth_user` id from the main database, which can't be referenced across
-- databases.
--
-- Instants are stored as RFC 3339 UTC text, so they compare correctly as strings.
-- AUTOINCREMENT keeps ids from ever being reused: a booking's id is its UID in the
-- published iCalendar feeds.

CREATE TABLE room (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#3b82f6'
);

CREATE TABLE booking (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL REFERENCES room(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL,
    -- `auth_user.id` in the main database; NULL for events imported from Google Calendar.
    created_by INTEGER,
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    -- UID of the matching Google Calendar event, if any.
    google_uid TEXT UNIQUE,
    CHECK (end_at > start_at)
);

CREATE INDEX booking_room_time ON booking (room_id, start_at, end_at);
CREATE INDEX booking_time ON booking (start_at, end_at);
