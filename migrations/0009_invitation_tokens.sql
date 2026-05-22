CREATE TABLE portal_invitation_tokens (
    id SERIAL PRIMARY KEY,
    email VARCHAR(254) NOT NULL,
    invited_by INTEGER NOT NULL REFERENCES auth_user(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);
CREATE INDEX idx_pit_token ON portal_invitation_tokens(token);
CREATE INDEX idx_pit_email ON portal_invitation_tokens(email);
