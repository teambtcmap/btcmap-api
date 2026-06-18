# get_request_log

## Description

Returns recent API request logs from the last N minutes. Useful for debugging and monitoring live traffic.

## Params

```json
{
  "minutes": 5
}
```

All fields are optional and have defaults:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `minutes` | integer | 1 | Number of minutes to look back |

## Result Format

```json
{
  "requests": [
    {
      "id": 12345,
      "date": "2024-01-15T10:30:00.123456Z",
      "ip": "192.168.1.1",
      "user_agent": "Mozilla/5.0 ...",
      "user_id": 42,
      "method": "POST",
      "path": "/rpc",
      "query": null,
      "body": "{\"jsonrpc\":\"2.0\",...}",
      "response_code": 200,
      "processing_time_ns": 1500000
    }
  ]
}
```

## Fields

- `id`: Unique request identifier
- `date`: ISO 8601 timestamp of the request
- `ip`: Client IP address
- `user_agent`: Client user agent string
- `user_id`: Authenticated user ID (null if anonymous)
- `method`: HTTP method (GET, POST, etc.)
- `path`: Request path
- `query`: Query string (null if none)
- `body`: Request body (null if none)
- `response_code`: HTTP response code
- `processing_time_ns`: Request processing time in nanoseconds

## Allowed Roles

- Root

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_request_log","params":{"minutes":5},"id":1}' \
  https://api.btcmap.org/rpc
```