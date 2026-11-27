-- A few words about the place, written by members ("Formule midi à 11 €, terrasse au soleil").
ALTER TABLE place ADD COLUMN description TEXT NOT NULL DEFAULT '';

-- Whether vegetarians find something good to eat there.
ALTER TABLE place ADD COLUMN veggie_friendly INTEGER NOT NULL DEFAULT 0;

-- The two seeded places known for it: a vegetarian buffet and a vegan kebab.
UPDATE place SET veggie_friendly = 1
WHERE name IN ('L''oignon givré', 'KVB') AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
