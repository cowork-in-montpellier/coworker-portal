-- A phone number to call the place, e.g. to order ahead. Kept as typed in ("04 67 12 34 56").
ALTER TABLE place ADD COLUMN phone TEXT;

-- Numbers of the seeded places, from their Google Maps pages (2026-11-27). Only seeded places
-- nobody has edited yet are touched (updated_by IS NULL). Jimmy Gourmet's is the craftsman's own
-- number, not necessarily reachable at the Albert 1er market stand.
UPDATE place SET phone = '04 67 17 48 52' WHERE name = 'Faitout' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 54 31 00 82' WHERE name = 'Tiger Noodle' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 76 84 89 61' WHERE name = 'Jimmy Gourmet' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 34 00 13 95' WHERE name = 'Chez Gaby' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 28 68 59 34' WHERE name = 'Big Bahut' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 99 67 45 36' WHERE name = 'Habibi' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 51 25 27 76' WHERE name = 'Pistache et Chocolat' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 12 93 83 28' WHERE name = 'Noon' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 67 72 83 23' WHERE name = 'Kim Hi Traiteur' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 50 72 23 35' WHERE name = 'Boucherie Febeo' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 30 10 83 57' WHERE name = 'Puja' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '07 53 15 65 82' WHERE name = 'African Box' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 62 41 56 71' WHERE name = 'L''oignon givré' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 67 70 39 28' WHERE name = 'Nguyen Thi Kim Yen' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 52 17 06 05' WHERE name = 'Li''s Raviolis Chinois' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 31 68 81 65' WHERE name = 'KVB' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 99 49 97 71' WHERE name = 'Le Raffiné' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 81 99 47 95' WHERE name = 'Burgershop34' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '06 43 74 14 16' WHERE name = 'Le Berlinois Kebab' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '09 84 51 76 40' WHERE name = 'Le Tandoori' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 67 06 99 35' WHERE name = 'Le Snack El Mendil' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
UPDATE place SET phone = '04 67 64 83 64' WHERE name = 'Coco d''Asie' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
