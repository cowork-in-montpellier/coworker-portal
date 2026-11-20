-- How many dictionary words were still possible when each guess was made, computed once when the
-- game ends. NULL for unfinished games, and for older ones until first shown.
ALTER TABLE portal_sutom_attempt ADD COLUMN possible_counts INT[];
