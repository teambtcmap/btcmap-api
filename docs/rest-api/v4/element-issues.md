
# Element Issues API (v4)

The Element Issues API allows you to retrieve information about issues associated with map elements.

## Endpoints

### Get Element Issues List

```
GET /v4/element-issues
```

Retrieves a list of element issues that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | **Required**. Filter issues updated since this time (RFC3339 format). |
| `limit` | Integer | **Required**. Limit the number of issues returned. |

#### Response

```json
[
  {
    "id": 1,
    "element_id": 123456,
    "type": "closed",
    "created_at": "2023-02-10T12:00:00Z",
    "updated_at": "2023-02-15T00:00:00Z",
    "reporter_id": 789,
    "resolver_id": null,
    "resolved_at": null
  }
]
```

#### Example Request

```
GET /v4/element-issues?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get Element Issue by ID

```
GET /v4/element-issues/{id}
```

Retrieves a specific element issue by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The element issue ID |

#### Response

```json
{
  "id": 1,
  "element_id": 123456,
  "type": "closed",
  "created_at": "2023-02-10T12:00:00Z",
  "updated_at": "2023-02-15T00:00:00Z",
  "reporter_id": 789,
  "resolver_id": null,
  "resolved_at": null
}
```

#### Example Request

```
GET /v4/element-issues/1
```
