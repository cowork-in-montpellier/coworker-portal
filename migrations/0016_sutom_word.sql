CREATE TABLE portal_sutom_word (
    game_date DATE PRIMARY KEY,
    word TEXT NOT NULL,
    possible_words TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
