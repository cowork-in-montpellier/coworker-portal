-- Add guest_token to billjobs_bill for unauthenticated bill access
ALTER TABLE billjobs_bill ADD COLUMN IF NOT EXISTS guest_token UUID;
