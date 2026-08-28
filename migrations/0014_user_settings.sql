CREATE TABLE portal_user_settings (
    user_id         INTEGER     PRIMARY KEY REFERENCES auth_user(id) ON DELETE CASCADE,
    onsite_payment  BOOLEAN     NOT NULL DEFAULT false
);
