
# Events API (v2)

Endpoints for retrieving event data in the v2 API.

## Get Events

Retrieves a list of events.

```
GET /v2/events
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| limit     | int  | Maximum number of events to return |
| offset    | int  | Number of events to skip |

### Example Response

```json
{
  "events": [
    {
      "id": "1",
      "title": "Example Event",
      "description": "Description of the event",
      "location": {
        "lat": 51.5074,
        "lon": -0.1278
      },
      "start_time": "2023-01-01T10:00:00Z",
      "end_time": "2023-01-01T12:00:00Z",
      "created_at": "2022-12-01T00:00:00Z"
    }
  ],
  "total": 10
}
```

## Get Event by ID

Retrieves a specific event by its ID.

```
GET /v2/events/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | string | The ID of the event |

### Example Response

```json
{
  "id": "1",
  "title": "Example Event",
  "description": "Description of the event",
  "location": {
    "lat": 51.5074,
    "lon": -0.1278
  },
  "start_time": "2023-01-01T10:00:00Z",
  "end_time": "2023-01-01T12:00:00Z",
  "created_at": "2022-12-01T00:00:00Z"
}
```
