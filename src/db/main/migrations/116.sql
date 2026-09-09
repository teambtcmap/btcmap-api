ALTER TABLE place_report ADD COLUMN submitted_by INTEGER REFERENCES "user"(id);

DROP TRIGGER place_report_updated_at;
CREATE TRIGGER place_report_updated_at UPDATE OF place_id, origin_id, type, extra_fields, ticket_url, submitted_by, created_at, closed_at, deleted_at ON place_report
BEGIN
    UPDATE place_report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
