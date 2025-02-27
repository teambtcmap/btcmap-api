
# Areas API (v3)

Endpoints for retrieving area data in the v3 API. This version provides enhanced filtering and more detailed area information.

## Get Areas

Retrieves a list of areas with enhanced filtering.

```
GET /v3/areas
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return areas updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of areas to return |

### Example Response

```json
[
  {
    "id": 1,
    "name": "Example Area",
    "url_alias": "example-area",
    "bounds": {
      "min_lat": 51.4,
      "min_lon": -0.2,
      "max_lat": 51.6,
      "max_lon": -0.1
    },
    "tags": {
      "featured": true
    },
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z",
    "deleted_at": null
  }
]
```

## Get Area by ID

Retrieves a specific area by its ID with enhanced details.

```
GET /v3/areas/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the area |

### Example Response

```json
{
  "id": 1,
  "name": "Example Area",
  "url_alias": "example-area",
  "bounds": {
    "min_lat": 51.4,
    "min_lon": -0.2,
    "max_lat": 51.6,
    "max_lon": -0.1
  },
  "tags": {
    "featured": true
  },
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z",
  "deleted_at": null
}
```
