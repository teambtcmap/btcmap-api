
# Element Comments API (v3)

Endpoints for retrieving element comment data in the v3 API.

## Get Element Comments

Retrieves a list of element comments.

```
GET /v3/element-comments
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| element_id     | int    | Filter comments by element ID |
| updated_since  | string | Return comments updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of comments to return |

### Example Response

```json
[
  {
    "id": 1,
    "element_id": 123,
    "user_id": 456,
    "content": "This is a great place!",
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z",
    "deleted_at": null
  }
]
```

## Get Element Comment by ID

Retrieves a specific element comment by its ID.

```
GET /v3/element-comments/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the element comment |

### Example Response

```json
{
  "id": 1,
  "element_id": 123,
  "user_id": 456,
  "content": "This is a great place!",
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z",
  "deleted_at": null
}
```
