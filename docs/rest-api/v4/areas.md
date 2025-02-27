
# Areas API (v4)

The Areas API allows you to retrieve information about geographical areas in the BTCMap ecosystem.

## Endpoints

### Get Areas List

```
GET /v4/areas
```

Retrieves a list of areas that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | Optional. Filter areas updated since this time (RFC3339 format). |
| `limit` | Integer | Optional. Limit the number of areas returned. |

#### Response

```json
[
  {
    "id": 123,
    "name": "Berlin",
    "tags": {
      "type": "city",
      "country": "Germany"
    },
    "bounds": {
      "min_lat": 52.3,
      "min_lon": 13.0,
      "max_lat": 52.7,
      "max_lon": 13.8
    },
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

### Get Area by ID

```
GET /v4/areas/{id}
```

Retrieves a specific area by its ID or alias.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | String | The area ID or URL alias |

#### Response

```json
{
  "id": 123,
  "name": "Berlin",
  "tags": {
    "type": "city",
    "country": "Germany"
  },
  "bounds": {
    "min_lat": 52.3,
    "min_lon": 13.0,
    "max_lat": 52.7,
    "max_lon": 13.8
  },
  "updated_at": "2023-01-15T00:00:00Z"
}
```

## Examples

### Get all areas updated since January 2023 with a limit of 10

```
GET /v4/areas?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get a specific area

```
GET /v4/areas/berlin
```
