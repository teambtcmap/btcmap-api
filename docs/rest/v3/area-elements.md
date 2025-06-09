
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
# Area Elements API (v3)

The Area Elements API provides access to relationships between geographic areas and elements (locations) within those areas.

## Endpoints

### GET /v3/area-elements

Retrieves area-element mappings with options to filter by update time and limit results.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| updated_since | string | Only return area-elements updated since this timestamp (RFC3339 format) |
| limit | integer | Maximum number of results to return |

#### Example Response

```json
[
  {
    "id": "12345",
    "area_id": "789",
    "element_id": "456",
    "updated_at": "2023-05-15T14:30:00Z",
    "deleted_at": null
  }
]
```

### GET /v3/area-elements/{id}

Retrieves a specific area-element relationship by its ID.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | string | The ID of the area-element relationship |

#### Example Response

```json
{
  "id": "12345",
  "area_id": "789",
  "element_id": "456", 
  "updated_at": "2023-05-15T14:30:00Z",
  "deleted_at": null
}
```

## Notes

- Area-element relationships are generated automatically by the system based on the geographic boundaries of areas and the locations of elements.
- The generation of these relationships is an administrative function and not available through the public API.
