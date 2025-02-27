
# Area Elements API (v3)

Endpoints for retrieving area-element relationships data in the v3 API.

## Get Area Elements

Retrieves a list of area-element relationships.

```
GET /v3/area-elements
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| area_id        | int    | Filter by area ID |
| element_id     | int    | Filter by element ID |
| updated_since  | string | Return relationships updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of relationships to return |

### Example Response

```json
[
  {
    "id": 1,
    "area_id": 123,
    "element_id": 456,
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z",
    "deleted_at": null
  }
]
```

## Get Area Element by ID

Retrieves a specific area-element relationship by its ID.

```
GET /v3/area-elements/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the area-element relationship |

### Example Response

```json
{
  "id": 1,
  "area_id": 123,
  "element_id": 456,
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z",
  "deleted_at": null
}
```
