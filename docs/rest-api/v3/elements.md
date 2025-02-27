
# Elements API (v3)

Endpoints for retrieving element data in the v3 API. This version enhances the element API with additional filtering options and more detailed responses.

## Get Elements

Retrieves a list of elements with enhanced filtering.

```
GET /v3/elements
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return elements updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of elements to return |
| tags           | string | Comma-separated list of tags to filter by |
| bbox           | string | Bounding box to filter by (format: minLon,minLat,maxLon,maxLat) |

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
    "deleted_at": null
  }
]
```

## Get Element by ID

Retrieves a specific element by its ID with enhanced details.

```
GET /v3/elements/{id}
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
  "deleted_at": null
}
```
