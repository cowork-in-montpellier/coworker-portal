-- Final ranking of each completed month, frozen once by the nightly task. It only counts
-- games finished before the month ended; catch-ups played afterwards never change it.
CREATE TABLE portal_sutom_month_ranking (
    month DATE NOT NULL, -- first day of the month
    user_id INT NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    rank INT NOT NULL, -- 1 = winner of the month
    points DOUBLE PRECISION NOT NULL,
    games_played INT NOT NULL,
    avg_guesses DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (month, user_id)
);
