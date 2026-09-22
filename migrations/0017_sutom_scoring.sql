ALTER TABLE portal_sutom_word
    ADD COLUMN puzzle_number INT,
    ADD COLUMN par INT;

-- Backfill puzzle_number for rows inserted before this column existed.
UPDATE portal_sutom_word
SET puzzle_number = (game_date - DATE '2022-01-08') + 1
WHERE puzzle_number IS NULL;

ALTER TABLE portal_sutom_word ALTER COLUMN puzzle_number SET NOT NULL;

CREATE TABLE portal_sutom_attempt (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    game_date DATE NOT NULL REFERENCES portal_sutom_word(game_date) ON DELETE CASCADE,
    guesses TEXT[] NOT NULL DEFAULT '{}',
    score INT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, game_date)
);
