
# Element Issues API (v4)

Endpoints for retrieving element issue data in the v4 API.

## Get Element Issues

Retrieves a list of element issues.

```
GET /v4/element-issues
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| element_id     | int    | Filter issues by element ID |
| updated_since  | string | Return issues updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of issues to return |
| status         | string | Filter by issue status (open, closed, all) |
| type           | string | Filter by issue type |

### Example Response

```json
[
  {
    "id": 1,
    "element_id": 123,
    "type": "verification_needed",
    "status": "open",
    "description": "This place needs verification",
    "created_by": 456,
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z",
    "closed_at": null
  }
]
```

## Get Element Issue by ID

Retrieves a specific element issue by its ID.

```
GET /v4/element-issues/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the element issue |

### Example Response

```json
{
  "id": 1,
  "element_id": 123,
  "type": "verification_needed",
  "status": "open",
  "description": "This place needs verification",
  "created_by": 456,
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z",
  "closed_at": null,
  "comments": [
    {
      "id": 789,
      "user_id": 456,
      "content": "I will verify this next week",
      "created_at": "2023-01-02T00:00:00Z"
    }
  ]
}
```
