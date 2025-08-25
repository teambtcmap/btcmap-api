ALTER TABLE area ADD COLUMN bbox_west REAL NOT NULL DEFAULT -180;
ALTER TABLE area ADD COLUMN bbox_south REAL NOT NULL DEFAULT -90;
ALTER TABLE area ADD COLUMN bbox_east REAL NOT NULL DEFAULT 180;
ALTER TABLE area ADD COLUMN bbox_north REAL NOT NULL DEFAULT 90;

DROP TRIGGER area_updated_at;

CREATE TRIGGER area_updated_at UPDATE OF alias, bbox_west, bbox_south, bbox_east, bbox_north, tags, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX idx_area_bbox_west ON area(bbox_west);
CREATE INDEX idx_area_bbox_south ON area(bbox_south);
CREATE INDEX idx_area_bbox_east ON area(bbox_east);
CREATE INDEX idx_area_bbox_north ON area(bbox_north);
