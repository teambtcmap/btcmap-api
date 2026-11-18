CREATE TABLE osm_element_og (
    osm_type TEXT NOT NULL,
    osm_id INTEGER NOT NULL,
    image_data BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    PRIMARY KEY (osm_type, osm_id)
) STRICT;

CREATE INDEX osm_element_og_created_at ON osm_element_og(created_at);
