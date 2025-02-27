
# REST API

The BTCMap REST API provides access to various resources through standard HTTP methods.

## API Base URL

All API endpoints are relative to the base URL: `https://api.btcmap.org/`

## Versioned Endpoints

The API is versioned with prefixes `/v2/`, `/v3/`, and `/v4/`. Always use the latest stable version when possible.

## Common Query Parameters

Many endpoints share these common query parameters:

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `updated_since` | ISO 8601 datetime | **Yes** | Filter resources updated since this time (RFC3339 format, e.g. `2023-01-01T00:00:00Z`) |
| `limit` | Integer | **Yes** | Limit the number of resources returned |

## Available Endpoints

- [Elements API](v3/elements.md) - Access and manage map elements
- [Events API](v3/events.md) - Access events related to map elements
- [Users API](v3/users.md) - Access user information
- [Areas API](v3/areas.md) - Access geographical area information
- [Element Issues API](v3/element-issues.md) - Retrieve issues associated with elements
- [Feeds API](v3/feeds.md) - Access feed information

## Error Handling

The API returns appropriate HTTP status codes:

| Status Code | Description |
|-------------|-------------|
| 200 | Success |
| 400 | Bad Request - Missing required parameters |
| 404 | Not Found - Resource not found |
| 500 | Internal Server Error |

## Rate Limiting

The API implements rate limiting to ensure fair usage. Clients should respect these limits and implement appropriate backoff strategies.
