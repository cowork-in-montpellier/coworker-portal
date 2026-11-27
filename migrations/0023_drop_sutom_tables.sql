-- SUTOM moved to its own SQLite database (see migrations_sutom/), where its data was copied
-- at startup. The old tables are no longer read or written.
DROP TABLE portal_sutom_month_ranking;
DROP TABLE portal_sutom_attempt;
DROP TABLE portal_sutom_word;
