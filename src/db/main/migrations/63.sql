CREATE TABLE boost(
    id INTEGER PRIMARY KEY NOT NULL,
    admin_id INTEGER NOT NULL REFERENCES admin(id),
    element_id INTEGER NOT NULL REFERENCES element(id),
    duration_days INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER boost_updated_at UPDATE OF admin_id, element_id, duration_days, created_at, deleted_at ON boost
BEGIN
    UPDATE boost SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;