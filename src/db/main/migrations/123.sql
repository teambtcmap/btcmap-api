-- User-authored notes: a free-form text pin at a coordinate, optionally made
-- public. Notes are a pure overlay and reference no place, area or other map
-- entity, only their author. A note is visible to everyone while `public = 1`
-- and to its author at all times.
CREATE TABLE note(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES "user"(id),
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    text TEXT NOT NULL,
    public INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE INDEX note_lat_lon ON note(lat, lon);
CREATE INDEX note_user_updated ON note(user_id, updated_at, id);
CREATE INDEX note_public ON note(public) WHERE deleted_at IS NULL;

CREATE TRIGGER note_updated_at
UPDATE OF user_id, lat, lon, text, public, created_at, deleted_at ON note
BEGIN
    UPDATE note SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
