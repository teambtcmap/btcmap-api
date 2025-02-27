
# Element Issues API

The Element Issues API provides access to issues reported for elements on the map.

## Endpoints

### GET /v4/element-issues

Retrieve a list of element issues that have been updated since a specified time.

#### Query Parameters

- `updated_since` (required): RFC3339 formatted datetime string
- `limit` (required): Maximum number of issues to return

#### Example Request

```
GET /v4/element-issues?updated_since=2023-01-01T00:00:00Z&limit=10
```

#### Example Response

```json
[
  {
    "id": 123,
    "element_id": 456,
    "user_id": 789,
    "issue_type": "closed_permanently",
    "description": "This place has been closed for several months",
    "status": "open",
    "updated_at": "2023-02-15T14:30:45Z"
  },
  {
    "id": 124,
    "element_id": 457,
    "user_id": 790,
    "issue_type": "incorrect_information",
    "description": "The hours listed are wrong - they open at 9am not 8am",
    "status": "resolved",
    "updated_at": "2023-02-16T09:12:33Z"
  }
]
```

### GET /v4/element-issues/{id}

Retrieve a specific element issue by its ID.

#### Path Parameters

- `id`: The unique identifier of the element issue

#### Example Request

```
GET /v4/element-issues/123
```

#### Example Response

```json
{
  "id": 123,
  "element_id": 456,
  "user_id": 789,
  "issue_type": "closed_permanently",
  "description": "This place has been closed for several months",
  "status": "open",
  "updated_at": "2023-02-15T14:30:45Z"
}
```
