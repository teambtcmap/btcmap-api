CREATE TABLE electrum_server(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL UNIQUE,
    priority INTEGER NOT NULL DEFAULT 0,
    spki_pin TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
ALTER TABLE conf DROP COLUMN electrum_url;
CREATE TRIGGER electrum_server_updated_at UPDATE OF name, url, priority, spki_pin, created_at, deleted_at ON electrum_server
BEGIN
    UPDATE electrum_server SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
