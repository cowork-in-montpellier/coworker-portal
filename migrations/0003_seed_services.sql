-- Seed app-owned service definitions.
-- external_service_id references billjobs_service.id (external billing system).
-- Multiple services can share the same external_service_id with different voucher specs.

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Full Time', 'Accès à temps plein pendant 1 mois',
       200.00, 'Monthly', NULL, NULL, s.id, true
FROM billjobs_service s WHERE s.reference = 'FT001'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Mid Time', 'Un accès 10 jours par mois au coworking',
       115.00, 'Monthly', NULL, NULL, s.id, true
FROM billjobs_service s WHERE s.reference = 'MT001'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Part Time', 'Accès 15 jours par mois à l''espace',
       145.00, 'Monthly', NULL, NULL, s.id, true
FROM billjobs_service s WHERE s.reference = 'PT002'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Carnet Journée', 'Carnet de 10 voucher de 10 hours',
       140.00, 'Book', 10, 10, s.id, true
FROM billjobs_service s WHERE s.reference = 'TD001'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Carnet Demi-journée', 'Carnet de 20 voucher de 5 hour',
       140.00, 'Book', 20, 5, s.id, true
FROM billjobs_service s WHERE s.reference = 'TD001'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Demi-journée', 'La demi-journée (5h)',
       140.00, 'Book', 1, 5, s.id, true
FROM billjobs_service s WHERE s.reference = 'OD005'
ON CONFLICT DO NOTHING;

INSERT INTO service (name, description, price, kind, amount, duration, external_service_id, is_available)
SELECT 'Journée', 'La journée (10h)',
       140.00, 'Book', 1, 10, s.id, true
FROM billjobs_service s WHERE s.reference = 'OD002'
ON CONFLICT DO NOTHING;