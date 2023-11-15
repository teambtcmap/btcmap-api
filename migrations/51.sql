ALTER TABLE token RENAME TO token_old;

CREATE TABLE token(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES user(id),
    secret TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL UNIQUE DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO token(
    id,
    user_id,
    secret,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    user_id,
    secret,
    created_at,
    updated_at,
    deleted_at
FROM token_old;

DROP TABLE token_old;

UPDATE token SET deleted_at = NULL WHERE deleted_at = '';

CREATE TRIGGER token_updated_at UPDATE OF user_id, secret, created_at, deleted_at ON token
BEGIN
    UPDATE token SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;