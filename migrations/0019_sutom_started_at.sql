-- When the player submitted their first guess, for the under-a-minute bonus.
-- Left NULL for attempts started before this column existed: they never get the bonus.
ALTER TABLE portal_sutom_attempt ADD COLUMN started_at TIMESTAMPTZ;
