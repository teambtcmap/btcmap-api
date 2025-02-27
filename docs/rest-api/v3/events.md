
# Events API (v3)

Endpoints for retrieving event data in the v3 API. This version provides enhanced filtering and more detailed event information.

## Get Events

Retrieves a list of events with enhanced filtering.

```
GET /v3/events
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return events updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of events to return |
| area_id        | int    | Filter events by area ID |

### Example Response

```json
[
  {
    "id": 1,
    "title": "Example Event",
    "description": "Description of the event",
    "element_id": 123,
    "user_id": 456,
    "start_time": "2023-01-01T10:00:00Z",
    "end_time": "2023-01-01T12:00:00Z",
    "tags": {
      "recurring": true,
      "bitcoin_only": true
    },
    "created_at": "2022-12-01T00:00:00Z",
    "updated_at": "2022-12-01T00:00:00Z",
    "deleted_at": null
  }
]
```

## Get Event by ID

Retrieves a specific event by its ID with enhanced details.

```
GET /v3/events/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the event |

### Example Response

```json
{
  "id": 1,
  "title": "Example Event",
  "description": "Description of the event",
  "element_id": 123,
  "user_id": 456,
  "start_time": "2023-01-01T10:00:00Z",
  "end_time": "2023-01-01T12:00:00Z",
  "tags": {
    "recurring": true,
    "bitcoin_only": true
  },
  "created_at": "2022-12-01T00:00:00Z",
  "updated_at": "2022-12-01T00:00:00Z",
  "deleted_at": null
}
```
