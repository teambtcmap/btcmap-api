# Places API (v4)

This document describes the endpoints for interacting with places in API v4.

## Available Endpoints

- [Get List](#get-list)
- [Get Single by ID](#get-by-id)
- [Get Comments](#get-comments)

### Get List

```
GET /v4/places
```

Retrieves a list of places. You can limit the output and apply a few useful filters.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `fields` | String | Optional. A comma-separated list of requested fields. |
| `updated_since` | ISO 8601 datetime | Optional. Filter places updated since this time. |
| `include_deleted` | Boolean | Optional. Whether to include deleted elements. Default is `false`. |
| `limit` | Integer | Optional. Limit the number of places returned. |

### Incremental Sync Approach (Recommended for Native Apps)

The `/v4/places` endpoint is designed for efficient incremental synchronization. Clients should:

1. Store the timestamp of their last sync locally
2. Request elements that have been updated since that timestamp using the `updated_since` parameter
3. Deleted places can be excluded during the first sync but you need to include deleted places for follow-up sync in order to invalidate previously cached but now deleted entries
4. Process only the changes since the last sync
5. Use `max(updated_at)` as a starting point for the follow-up sync jobs

This approach minimizes data transfer and processing requirements, making it ideal for mobile applications and other bandwidth-constrained environments.

#### Example Incremental Sync Flow:

```
// Initial sync - store returned timestamp
GET /v4/places?updated_since=2020-01-01T00:00:00Z&limit=1000

// Subsequent sync - use timestamp from previous response
GET /v4/places?updated_since=2023-09-15T14:30:45Z&limit=1000
```

#### Field Selection

The `fields` parameter allows you to request specific fields to be included in the response, which can improve performance for large requests.

Available fields include:

```
lat // Place latitude
lon // Place longitude
icon // Place icon
name // Place name
```

##### Examples:

Basic request for active places with location and name:
```
GET /v4/elements?fields=lat,lon,name&limit=5
```

#### Response

```json
TODO
```

### Get Single by ID

```
GET /v4/places/{id}
```

Retrieves a specific place by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The element ID |

#### Response

```json
TODO
```

#### Example Request

```
GET /v4/elements/123456
