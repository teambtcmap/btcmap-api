CREATE INDEX request_path_date_rest ON request(path, date) WHERE path LIKE '/v%' OR path LIKE '/feeds%';
ANALYZE request;
