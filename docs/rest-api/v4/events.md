# Events API (v4)

This document describes the endpoints for interacting with events in API v4.

## Available Endpoints

- [GET /v4/events](#get-v4events) - Retrieve events based on query parameters
- [GET /v4/events/{id}](#get-v4eventsid) - Retrieve a specific event by ID

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