
# Areas API (v3)

The Areas API allows you to retrieve information about geographic areas defined in the BTCMap platform.

## Endpoints

### Get Areas List

```
GET /v3/areas
```

Retrieves a list of geographic areas that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | **Required**. Filter areas updated since this time (RFC3339 format). |
| `limit` | Integer | **Required**. Limit the number of areas returned. |

#### Response

```json
[
  {
    "id": 123,
    "name": "New York City",
    "url_alias": "nyc",
    "osm_id": 175905,
    "osm_type": "relation",
    "tags": {
      "name": "New York City",
      "place": "city"
    },
    "bounds": {
      "min_lon": -74.25909,
      "min_lat": 40.477399,
      "max_lon": -73.700272,
      "max_lat": 40.916178
    },
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

#### Example Request

```
GET /v3/areas?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get Area by ID

```
GET /v3/areas/{id}
```

Retrieves a specific geographic area by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The area ID |

#### Response

```json
{
  "id": 123,
  "name": "New York City",
  "url_alias": "nyc",
  "osm_id": 175905,
  "osm_type": "relation",
  "tags": {
    "name": "New York City",
    "place": "city"
  },
  "bounds": {
    "min_lon": -74.25909,
    "min_lat": 40.477399,
    "max_lon": -73.700272,
    "max_lat": 40.916178
  },
  "updated_at": "2023-01-15T00:00:00Z"
}
```

#### Example Request

```
GET /v3/areas/123
```
