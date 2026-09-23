CREATE TABLE portal_sutom_word (
    game_date DATE PRIMARY KEY,
    word TEXT NOT NULL,
    puzzle_number INT NOT NULL,
    par INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE portal_sutom_attempt (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    game_date DATE NOT NULL REFERENCES portal_sutom_word(game_date) ON DELETE CASCADE,
    guesses TEXT[] NOT NULL DEFAULT '{}',
    score INT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, game_date)
);
