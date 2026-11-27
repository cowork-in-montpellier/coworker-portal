-- Pause-déj's own database, kept apart from the billing one. Members are referred to by
-- `auth_user` id from the main database, which can't be referenced across databases.
--
-- Instants are stored as RFC 3339 UTC text, dates as YYYY-MM-DD (Europe/Paris).

CREATE TABLE place (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    -- Free text ("Burgers", "Libanais"...), suggested from the types already in use.
    cuisine TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    -- The place's Google Maps page, if it has one.
    maps_url TEXT,
    -- Weekdays it is open for lunch, as a bit mask: Mon = 1, Tue = 2, Wed = 4, Thu = 8, Fri = 16.
    open_days INTEGER NOT NULL DEFAULT 31 CHECK (open_days BETWEEN 1 AND 31),
    -- The one "J'ai ma gamelle" entry, for those who brought their own lunch.
    is_gamelle INTEGER NOT NULL DEFAULT 0,
    -- `auth_user.id` in the main database; NULL for seeded entries.
    created_by INTEGER,
    created_at TEXT NOT NULL,
    updated_by INTEGER,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX place_single_gamelle ON place (is_gamelle) WHERE is_gamelle = 1;

-- A member's rating of a place, one per member and place.
CREATE TABLE rating (
    place_id INTEGER NOT NULL REFERENCES place (id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL,
    stars INTEGER NOT NULL CHECK (stars BETWEEN 1 AND 5),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (place_id, user_id)
);

-- Where a member means to have lunch on a given day: one place per member and day.
CREATE TABLE intention (
    user_id INTEGER NOT NULL,
    lunch_date TEXT NOT NULL,
    place_id INTEGER NOT NULL REFERENCES place (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, lunch_date)
);

CREATE INDEX intention_date ON intention (lunch_date);
