CREATE TABLE area_element(
    id INTEGER PRIMARY KEY NOT NULL,
    area_id INTEGER NOT NULL REFERENCES area(id),
    element_id INTEGER NOT NULL REFERENCES element(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER area_element_updated_at UPDATE OF area_id, element_id, created_at, deleted_at ON area_element
BEGIN
    UPDATE area_element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX area_element_area_id ON area_element(area_id);

CREATE INDEX area_element_element_id ON area_element(element_id);