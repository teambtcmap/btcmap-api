-- Track when each access token was last presented for authentication so
-- unused or leaked tokens can be spotted. Existing tokens default to the Unix
-- epoch, meaning "never used". `last_used_at` is deliberately kept out of the
-- `acess_token_updated_at` trigger: a use is not a metadata change, so
-- `updated_at` keeps meaning "when the token record last changed".
ALTER TABLE access_token ADD COLUMN last_used_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00.000Z';
