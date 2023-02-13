CREATE TABLE token (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    secret TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TRIGGER token_updated_at UPDATE OF user_id, secret, created_at, deleted_at ON token
BEGIN
    UPDATE token SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;