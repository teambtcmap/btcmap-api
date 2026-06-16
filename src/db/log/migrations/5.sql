ALTER TABLE request ADD COLUMN rpc_method TEXT GENERATED ALWAYS AS (CASE WHEN json_valid(body) THEN json_extract(body, '$.method') END);
CREATE INDEX request_rpc_method ON request(rpc_method, date) WHERE path = '/rpc';
ANALYZE request;
