ALTER TABLE portal_room_booking
    ALTER COLUMN created_by DROP NOT NULL,
    ADD COLUMN google_uid TEXT UNIQUE;