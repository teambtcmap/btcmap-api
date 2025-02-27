
# Events API (v4)

The Events API allows you to retrieve information about events that occur on the BTCMap platform, such as element creations, updates, and deletions.

## Endpoints

### Get Events List

```
GET /v4/events
```

Retrieves a list of events that have occurred since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | Optional. Filter events that occurred since this time (RFC3339 format). |
| `limit` | Integer | Optional. Limit the number of events returned. |
| `type` | String | Optional. Filter by event type (create, update, delete). |
| `element_id` | Integer | Optional. Filter events related to a specific element. |

#### Response

```json
[
  {
    "id": 123,
    "type": "create",
    "element_id": 456,
    "user_id": 789,
    "created_at": "2023-01-15T00:00:00Z",
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

### Get Event by ID

```
GET /v4/events/{id}
```

Retrieves a specific event by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | The event ID |

#### Response

```json
{
  "id": 123,
  "type": "create",
  "element_id": 456,
  "user_id": 789,
  "created_at": "2023-01-15T00:00:00Z",
  "updated_at": "2023-01-15T00:00:00Z"
}
```

## Examples

### Get all creation events since January 2023 with a limit of 10

```
GET /v4/events?updated_since=2023-01-01T00:00:00Z&type=create&limit=10
```

### Get a specific event

```
GET /v4/events/123
```
