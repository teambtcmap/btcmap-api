
# Area Elements API

The Area Elements API provides access to elements within specific geographic areas.

## Endpoints

### GET /v3/area-elements

Retrieve a list of area elements that have been updated since a specified time.

#### Query Parameters

- `updated_since` (required): RFC3339 formatted datetime string
- `limit` (required): Maximum number of area elements to return

#### Example Request

```
GET /v3/area-elements?updated_since=2023-01-01T00:00:00Z&limit=10
```

#### Example Response

```json
[
  {
    "id": 123,
    "area_id": 456,
    "element_id": 789,
    "updated_at": "2023-02-15T14:30:45Z"
  },
  {
    "id": 124,
    "area_id": 457,
    "element_id": 790,
    "updated_at": "2023-02-16T09:12:33Z"
  }
]
```

### GET /v3/area-elements/{id}

Retrieve a specific area element by its ID.

#### Path Parameters

- `id`: The unique identifier of the area element

#### Example Request

```
GET /v3/area-elements/123
```

#### Example Response

```json
{
  "id": 123,
  "area_id": 456,
  "element_id": 789,
  "updated_at": "2023-02-15T14:30:45Z"
}
```
