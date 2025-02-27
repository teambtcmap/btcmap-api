
# Elements API (v2)

Endpoints for retrieving element data in the v2 API.

## Get Elements

Retrieves a list of elements.

```
GET /v2/elements
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| limit     | int  | Maximum number of elements to return |
| offset    | int  | Number of elements to skip |

### Example Response

```json
{
  "elements": [
    {
      "id": "1",
      "name": "Example Element",
      "description": "Description of the element",
      "tags": ["tag1", "tag2"],
      "location": {
        "lat": 51.5074,
        "lon": -0.1278
      },
      "created_at": "2023-01-01T00:00:00Z",
      "updated_at": "2023-01-01T00:00:00Z"
    }
  ],
  "total": 100
}
```

## Get Element by ID

Retrieves a specific element by its ID.

```
GET /v2/elements/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | string | The ID of the element |

### Example Response

```json
{
  "id": "1",
  "name": "Example Element",
  "description": "Description of the element",
  "tags": ["tag1", "tag2"],
  "location": {
    "lat": 51.5074,
    "lon": -0.1278
  },
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z"
}
```
