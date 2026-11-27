-- Google Maps links, addresses, cuisines and lunch days of the seeded places, gathered with
-- a web search (2026-11-27). Only seeded places nobody has edited yet are touched
-- (updated_by IS NULL). Jimmy Gourmet only gets its link and keeps the market's address and days.

UPDATE place SET maps_url = 'https://maps.app.goo.gl/mVaJBMSX5g4hZyVS7', address = '4 boulevard Pasteur, 34090 Montpellier', cuisine = 'Cuisine variée', open_days = 31
WHERE name = 'Faitout' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/fXzBMJExESBFjZM57', address = '4 boulevard Louis Blanc, 34000 Montpellier', cuisine = 'Chinois', open_days = 31
WHERE name = 'Tiger Noodle' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/EM6SX3VHscUNbn6x9', address = '8 place des Beaux-Arts, 34000 Montpellier', cuisine = 'Burgers', open_days = 30, under_ten_euros = 1
WHERE name = 'Big Bahut' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/ypgW9GHQte4wN5k56', address = '57 rue de l''Aiguillerie, 34000 Montpellier', cuisine = 'Libanais', open_days = 27
WHERE name = 'Habibi' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/6vFgsYgW7rau9F5v8', address = '9 rue Proudhon, 34000 Montpellier', cuisine = 'Boulangerie', open_days = 31
WHERE name = 'Pistache et Chocolat' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/3HVm9Gg6D7kPeBti9', address = '4 bis rue Bernard Délicieux, 34000 Montpellier', cuisine = 'Snack', open_days = 31
WHERE name = 'Noon' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/k9SJyezgZHkewBiAA', address = '5 rue Proudhon, 34000 Montpellier', cuisine = 'Traiteur asiatique', open_days = 31
WHERE name = 'Kim Hi Traiteur' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/gJxsUBa7EdEMsWg38', address = '20 place Émile Combes, 34000 Montpellier', cuisine = 'Boucherie', open_days = 31
WHERE name = 'Boucherie Febeo' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/z6ghz8qg5r4aQqji9', address = '3 rue Ferdinand Fabre, 34090 Montpellier', cuisine = 'Indien', open_days = 31
WHERE name = 'Puja' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/BU7ZfsUAJoCLZpp38', address = '13 rue des Écoles Laïques, 34000 Montpellier', cuisine = 'Africain', open_days = 31
WHERE name = 'African Box' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/t3hZ6akBp2F9YScHA', address = '46 rue de l''Université, 34000 Montpellier', cuisine = 'Végétarien', open_days = 31, under_ten_euros = 1
WHERE name = 'L''oignon givré' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/2rS71zQEZpdXwwfU9', address = '48 rue de l''Université, 34000 Montpellier', cuisine = 'Vietnamien', open_days = 31
WHERE name = 'Nguyen Thi Kim Yen' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/4RRJc4ab8QYAcAKa8', address = '23 rue de l''Université, 34000 Montpellier', cuisine = 'Chinois', open_days = 31
WHERE name = 'Li''s Raviolis Chinois' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/gay945b4hNT5HV5o7', address = '71 rue de l''Aiguillerie, 34000 Montpellier', cuisine = 'Kebab', open_days = 28
WHERE name = 'KVB' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/7UZrMTQNGzCDrPra7', address = '48 rue de l''Aiguillerie, 34000 Montpellier', cuisine = 'Kebab', open_days = 31
WHERE name = 'Le Raffiné' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/Dv8pU8w6cGV2ZfT68', address = '65 rue de l''Aiguillerie, 34000 Montpellier', cuisine = 'Burgers', open_days = 31
WHERE name = 'Burgershop34' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/ceoKQ1ZUwd4JkL5YA', address = '1 rue Sainte-Ursule, 34000 Montpellier', cuisine = 'Kebab', open_days = 24
WHERE name = 'Le Berlinois Kebab' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/pLZ8Q88N5UNA2mpW8', address = '7 rue des Écoles Laïques, 34000 Montpellier', cuisine = 'Indien', open_days = 31
WHERE name = 'Le Tandoori' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/aU7EsGCJHoSoe3gMA', address = '5 avenue de Lodève, 34000 Montpellier', cuisine = 'Snack', open_days = 31
WHERE name = 'Le Snack El Mendil' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/kwnDBWXU8hwbQNww5', address = '4 rue Ernest Michel, 34000 Montpellier', cuisine = 'Asiatique', open_days = 31
WHERE name = 'Coco d''Asie' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

UPDATE place SET maps_url = 'https://maps.app.goo.gl/ydMrGBmenK9wAofN9'
WHERE name = 'Jimmy Gourmet' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;

-- « Gabriel Créateur de Gourmandises », pâtisserie closed on Mondays.
UPDATE place SET maps_url = 'https://maps.app.goo.gl/4U8VTxjocD43yitLA', address = '16 boulevard Louis Blanc, 34000 Montpellier', cuisine = 'Boulangerie', open_days = 30
WHERE name = 'Chez Gaby' AND is_gamelle = 0 AND created_by IS NULL AND updated_by IS NULL;
