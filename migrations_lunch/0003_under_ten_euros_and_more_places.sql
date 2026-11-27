-- Whether one can have lunch there for less than 10 €.
ALTER TABLE place ADD COLUMN under_ten_euros INTEGER NOT NULL DEFAULT 0;

-- More places the coworkers go to. Cuisines are best guesses from the names, left empty when
-- unknown; addresses, Maps links and days are completed later or by members from the page.
INSERT INTO place (name, cuisine, created_at, updated_at) VALUES
    ('Noon', '', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Kim Hi Traiteur', 'Traiteur asiatique', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Boucherie Febeo', 'Boucherie', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Puja', 'Indien', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('African Box', 'Africain', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('L''oignon givré', '', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Nguyen Thi Kim Yen', 'Vietnamien', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Li''s Raviolis Chinois', 'Chinois', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('KVB', '', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Le Raffiné', '', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Burgershop34', 'Burgers', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Le Berlinois Kebab', 'Kebab', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Le Tandoori', 'Indien', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Le Snack El Mendil', 'Snack', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z'),
    ('Coco d''Asie', 'Asiatique', '2026-11-27T00:00:00Z', '2026-11-27T00:00:00Z');
