CREATE TABLE request (
    id INTEGER PRIMARY KEY NOT NULL,
    date TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    ip TEXT NOT NULL,
    user_agent TEXT,
    user_id INTEGER,
    path TEXT NOT NULL, 
    query TEXT,
    body TEXT,
    response_code INTEGER NOT NULL,
    processing_time_ns INTEGER NOT NULL
) STRICT;

CREATE INDEX request_date ON request(date);
CREATE INDEX request_ip ON request(ip);
CREATE INDEX request_user_agent ON request(user_agent);
CREATE INDEX request_user_id ON request(user_id);
CREATE INDEX request_path ON request(path);
CREATE INDEX request_response_code ON request(response_code);