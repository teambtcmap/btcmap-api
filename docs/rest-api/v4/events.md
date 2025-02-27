
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
# Events API (v4)

The Events API provides access to Bitcoin and cryptocurrency-related events.

## Endpoints

### GET /v4/events

Retrieves a list of events with optional filtering.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| updated_since | string | Only return events updated since this timestamp (RFC3339 format) |
| limit | integer | Maximum number of results to return |
| start_date | string | Only return events starting on or after this date (YYYY-MM-DD) |
| end_date | string | Only return events ending on or before this date (YYYY-MM-DD) |
| area_id | string | Only return events in this area |

#### Example Response

```json
[
  {
    "id": "789",
    "name": "Bitcoin Meetup",
    "description": "Weekly Bitcoin meetup for enthusiasts",
    "start_date": "2023-09-15T18:00:00Z",
    "end_date": "2023-09-15T20:00:00Z",
    "location": {
      "name": "Bitcoin Cafe",
      "address": "123 Crypto St, San Francisco, CA",
      "coordinates": [-122.419, 37.774]
    },
    "organizer": {
      "name": "SF Bitcoin Devs",
      "contact": "info@sfbitcoindevs.org"
    },
    "url": "https://sfbitcoindevs.org/meetup/2023-09-15",
    "updated_at": "2023-08-01T14:30:00Z",
    "deleted_at": null
  }
]
```

### GET /v4/events/{id}

Retrieves a specific event by its ID.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | string | The ID of the event |

#### Example Response

```json
{
  "id": "789",
  "name": "Bitcoin Meetup",
  "description": "Weekly Bitcoin meetup for enthusiasts",
  "start_date": "2023-09-15T18:00:00Z",
  "end_date": "2023-09-15T20:00:00Z",
  "location": {
    "name": "Bitcoin Cafe",
    "address": "123 Crypto St, San Francisco, CA",
    "coordinates": [-122.419, 37.774]
  },
  "organizer": {
    "name": "SF Bitcoin Devs",
    "contact": "info@sfbitcoindevs.org"
  },
  "url": "https://sfbitcoindevs.org/meetup/2023-09-15",
  "updated_at": "2023-08-01T14:30:00Z",
  "deleted_at": null
}
```
