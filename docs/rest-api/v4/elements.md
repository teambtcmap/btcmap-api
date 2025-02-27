# Elements API (v4)

This document describes the endpoints for interacting with elements in API v4.

## Available Endpoints

- [Get Elements List](#get-elements-list)
- [Get Element by ID](#get-element-by-id)

### Get Elements List

```
GET /v4/elements
```

Retrieves a list of elements that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | **Required**. Filter elements updated since this time (RFC3339 format). |
| `limit` | Integer | **Required**. Limit the number of elements returned. |

#### Response

```json
[
  {
    "id": 123456,
    "osm_type": "node",
    "osm_id": 123456,
    "geolocation": {
      "latitude": 40.7128,
      "longitude": -74.0060
    },
    "tags": {
      "name": "Bitcoin Coffee",
      "amenity": "cafe",
      "currency:XBT": "yes"
    },
    "issues": [
      {
        "id": 1,
        "type": "closed",
        "created_at": "2023-02-10T12:00:00Z"
      }
    ],
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

#### Example Request

```
GET /v4/elements?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get Element by ID

```
GET /v4/elements/{id}
```

Retrieves a specific element by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The element ID |

#### Response

```json
{
  "id": 123456,
  "osm_type": "node",
  "osm_id": 123456,
  "geolocation": {
    "latitude": 40.7128,
    "longitude": -74.0060
  },
  "tags": {
    "name": "Bitcoin Coffee",
    "amenity": "cafe",
    "currency:XBT": "yes"
  },
  "issues": [
    {
      "id": 1,
      "type": "closed",
      "created_at": "2023-02-10T12:00:00Z"
    }
  ],
  "updated_at": "2023-01-15T00:00:00Z"
}
```

#### Example Request

```
GET /v4/elements/123456