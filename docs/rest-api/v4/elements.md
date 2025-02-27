
# Elements API (v4)

Endpoints for retrieving element data in the v4 API. This version provides the most advanced filtering and query capabilities.

## Get Elements

Retrieves a list of elements with advanced filtering options.

```
GET /v4/elements
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return elements updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of elements to return |
| tags           | string | Comma-separated list of tags to filter by |
| bbox           | string | Bounding box to filter by (format: minLon,minLat,maxLon,maxLat) |
| status         | string | Filter by element status (active, deleted, all) |
| has_issues     | bool   | Filter elements that have open issues |

### Example Response

```json
[
  {
    "id": 1,
    "osm_type": "node",
    "osm_id": 123456789,
    "tags": {
      "name": "Example Element",
      "amenity": "cafe",
      "currency:BTC": "yes"
    },
    "lon": -0.1278,
    "lat": 51.5074,
    "updated_at": "2023-01-01T00:00:00Z",
    "deleted_at": null,
    "issues_count": 0,
    "last_verified": "2023-01-15T00:00:00Z"
  }
]
```

## Get Element by ID

Retrieves a specific element by its ID with comprehensive details.

```
GET /v4/elements/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the element |

### Example Response

```json
{
  "id": 1,
  "osm_type": "node",
  "osm_id": 123456789,
  "tags": {
    "name": "Example Element",
    "amenity": "cafe",
    "currency:BTC": "yes"
  },
  "lon": -0.1278,
  "lat": 51.5074,
  "updated_at": "2023-01-01T00:00:00Z",
  "deleted_at": null,
  "issues": [
    {
      "id": 1,
      "type": "verification_needed",
      "created_at": "2023-01-10T00:00:00Z"
    }
  ],
  "history": [
    {
      "timestamp": "2022-12-01T00:00:00Z",
      "user_id": 123,
      "change_type": "created"
    },
    {
      "timestamp": "2023-01-01T00:00:00Z",
      "user_id": 456,
      "change_type": "updated"
    }
  ],
  "last_verified": "2023-01-15T00:00:00Z"
}
```
