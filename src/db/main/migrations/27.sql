CREATE TABLE admin_action (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    message TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

CREATE TRIGGER admin_action_updated_at UPDATE OF user_id, message, created_at, deleted_at ON admin_action
BEGIN
    UPDATE admin_action SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;