-- The places the coworkers usually go to, around 7 rue de Villefranche. Only what could be
-- checked is filled in: members complete the rest (addresses, Maps links, days) from the page.
-- Open days default to Monday to Friday (31); the Albert 1er market runs Wednesday and Friday (4 + 16).

INSERT INTO place (name, cuisine, address, open_days, is_gamelle, created_at, updated_at) VALUES
    ('J''ai ma gamelle', 'Fait maison', '', 31, 1, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Faitout', 'Cuisine variée', '4 boulevard Pasteur, 34000 Montpellier', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Tiger Noodle', 'Chinois', '4 boulevard Louis Blanc, 34000 Montpellier', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Jimmy Gourmet', 'Marché', 'Marché Albert 1er, place Albert 1er, 34000 Montpellier', 20, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Chez Gaby', '', '', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Big Bahut', 'Burgers', '', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Habibi', 'Libanais', '', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Pistache et Chocolat', 'Boulangerie', '9 rue Proudhon, 34000 Montpellier', 31, 0, '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z');
