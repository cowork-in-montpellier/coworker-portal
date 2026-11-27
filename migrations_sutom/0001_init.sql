-- SUTOM's own database, kept apart from the billing one. Players are `auth_user` ids from
-- the main database, which can't be referenced across databases: a player's games are
-- simply never shown once their account is gone.
--
-- Dates are stored as `YYYY-MM-DD` and instants as RFC 3339 UTC text, so both compare
-- correctly as strings.

CREATE TABLE word (
    game_date TEXT PRIMARY KEY,
    word TEXT NOT NULL,
    puzzle_number INTEGER NOT NULL,
    -- Frozen by the nightly task once the day is over; NULL until then.
    par INTEGER
);

CREATE TABLE attempt (
    user_id INTEGER NOT NULL,
    game_date TEXT NOT NULL REFERENCES word(game_date) ON DELETE CASCADE,
    -- JSON array of the guesses, in order.
    guesses TEXT NOT NULL DEFAULT '[]',
    -- Number of guesses to find the word, 7 if never found, NULL while in progress.
    score INTEGER,
    -- JSON array: how many dictionary words were still possible when each guess was made,
    -- computed once when the game ends. NULL until then, and for older games until first shown.
    possible_counts TEXT,
    -- When the first guess was submitted, for the under-a-minute bonus. NULL for games
    -- started before it was recorded: they never get the bonus.
    started_at TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, game_date)
);

-- Final ranking of each completed month, frozen once by the nightly task. It only counts
-- games finished before the month ended; catch-ups played afterwards never change it.
CREATE TABLE month_ranking (
    month TEXT NOT NULL, -- first day of the month
    user_id INTEGER NOT NULL,
    rank INTEGER NOT NULL, -- 1 = winner of the month
    points REAL NOT NULL,
    games_played INTEGER NOT NULL,
    avg_guesses REAL NOT NULL,
    PRIMARY KEY (month, user_id)
);
