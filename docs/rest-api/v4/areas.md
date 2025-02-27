# Areas API (v4)

This document describes the endpoints for interacting with areas in API v4.

## Available Endpoints

- [GET /v4/areas](#get-v4areas) - Retrieve areas based on query parameters
- [GET /v4/areas/{id}](#get-v4areasid) - Retrieve a specific area by ID

## Endpoints

### GET /v4/areas

Retrieves a list of areas with optional filtering.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| updated_since | string | Only return areas updated since this timestamp (RFC3339 format) |
| limit | integer | Maximum number of results to return |

#### Example Response

```json
[
  {
    "id": "123",
    "name": "San Francisco",
    "geometry": {
      "type": "Polygon",
      "coordinates": [
        [
          [-122.51, 37.71],
          [-122.51, 37.83],
          [-122.36, 37.83],
          [-122.36, 37.71],
          [-122.51, 37.71]
        ]
      ]
    },
    "properties": {
      "country": "United States",
      "continent": "North America"
    },
    "updated_at": "2023-06-15T14:30:00Z",
    "deleted_at": null
  }
]
```

### GET /v4/areas/{id}

Retrieves a specific area by its ID.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | string | The ID of the area |

#### Example Response

```json
{
  "id": "123",
  "name": "San Francisco",
  "geometry": {
    "type": "Polygon",
    "coordinates": [
      [
        [-122.51, 37.71],
        [-122.51, 37.83],
        [-122.36, 37.83],
        [-122.36, 37.71],
        [-122.51, 37.71]
      ]
    ]
  },
  "properties": {
    "country": "United States",
    "continent": "North America"
  },
  "updated_at": "2023-06-15T14:30:00Z",
  "deleted_at": null
}