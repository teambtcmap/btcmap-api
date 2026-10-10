CREATE INDEX IF NOT EXISTS request_date_user_id ON request(date, user_id) WHERE user_id IS NOT NULL;
ANALYZE request_date_user_id;
