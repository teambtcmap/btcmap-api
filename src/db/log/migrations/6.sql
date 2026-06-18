ALTER TABLE request ADD COLUMN method TEXT NOT NULL DEFAULT '';
CREATE INDEX request_method_date ON request(method, date);
ANALYZE request;
