-- Speed up the per-area report endpoint (GET /v4/areas/{id}/reports), which
-- reads one area's rows ordered by date. The report table is one row per area
-- per changed day; the only existing index covers updated_at, which the sync
-- queries use. This composite index lets the endpoint satisfy both the
-- area_id filter and the ORDER BY date, id without a full scan.
CREATE INDEX report_area_id_date ON report(area_id, date);
