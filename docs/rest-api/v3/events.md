
# Events API (v3)

The Events API allows you to retrieve information about events that occur on the BTCMap platform.

## Endpoints

### Get Events List

```
GET /v3/events
```

Retrieves a list of events that have occurred since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `created_since` | ISO 8601 datetime | **Required**. Filter events created since this time (RFC3339 format). |
| `limit` | Integer | **Required**. Limit the number of events returned. |

#### Response

```json
[
  {
    "id": 123,
    "type": "element_added",
    "element_id": 456,
    "user_id": 789,
    "created_at": "2023-01-15T00:00:00Z",
    "data": {
      "name": "Bitcoin Cafe",
      "tags": {
        "name": "Bitcoin Cafe",
        "amenity": "cafe",
        "currency:XBT": "yes"
      }
    }
  }
]
```

#### Example Request

```
GET /v3/events?created_since=2023-01-01T00:00:00Z&limit=10
```

### Get Event by ID

```
GET /v3/events/{id}
```

Retrieves a specific event by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The event ID |

#### Response

```json
{
  "id": 123,
  "type": "element_added",
  "element_id": 456,
  "user_id": 789,
  "created_at": "2023-01-15T00:00:00Z",
  "data": {
    "name": "Bitcoin Cafe",
    "tags": {
      "name": "Bitcoin Cafe",
      "amenity": "cafe",
      "currency:XBT": "yes"
    }
  }
}
```

#### Example Request

```
GET /v3/events/123
```
