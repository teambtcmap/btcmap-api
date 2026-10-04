-- Make `starts_at` mandatory. Events without a start date have been removed
-- from every environment, so the rebuild cannot hit an invalid row. SQLite
-- cannot change a column's nullability in place, so the table is rebuilt and
-- its indexes/trigger recreated.
DROP TRIGGER event_updated_at;

CREATE TABLE event_new(
    id INTEGER PRIMARY KEY NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    name TEXT NOT NULL,
    website TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT,
    area_id INTEGER REFERENCES area(id)
) STRICT;

INSERT INTO event_new (id, lat, lon, name, website, starts_at, ends_at, created_at, updated_at, deleted_at, area_id)
SELECT id, lat, lon, name, website, starts_at, ends_at, created_at, updated_at, deleted_at, area_id
FROM event;

DROP TABLE event;
ALTER TABLE event_new RENAME TO event;

CREATE INDEX event_lat_lon ON event(lat, lon);
CREATE INDEX event_updated_at ON event(updated_at, id);
CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, area_id, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
