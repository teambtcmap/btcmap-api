
# Areas API (v2)

Endpoints for retrieving area data in the v2 API.

## Get Areas

Retrieves a list of areas.

```
GET /v2/areas
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| limit     | int  | Maximum number of areas to return |
| offset    | int  | Number of areas to skip |

### Example Response

```json
{
  "areas": [
    {
      "id": "1",
      "name": "Example Area",
      "bounds": {
        "min_lat": 51.4,
        "min_lon": -0.2,
        "max_lat": 51.6,
        "max_lon": -0.1
      },
      "created_at": "2023-01-01T00:00:00Z"
    }
  ],
  "total": 5
}
```

## Get Area by URL Alias

Retrieves a specific area by its URL alias.

```
GET /v2/areas/{url_alias}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| url_alias | string | The URL alias of the area |

### Example Response

```json
{
  "id": "1",
  "name": "Example Area",
  "url_alias": "example-area",
  "bounds": {
    "min_lat": 51.4,
    "min_lon": -0.2,
    "max_lat": 51.6,
    "max_lon": -0.1
  },
  "created_at": "2023-01-01T00:00:00Z"
}
```
