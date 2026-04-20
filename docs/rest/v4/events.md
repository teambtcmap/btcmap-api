# Events REST API (v4)

This document describes the endpoints for interacting with events in REST API v4.

## Available Endpoints

- [Get Batch](#get-list)
- [Get by ID](#get-by-id)

### Get Batch

```bash
curl --request GET https://api.btcmap.org/v4/events
```

Retrieves a list of all non-deleted events with future start dates. Events without a start date are also included.

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | Integer | Unique identifier for the event. |
| `area_id` | Integer or null | The area ID this event belongs to, if any. |
| `lat` | Float | Latitude of the event location. |
| `lon` | Float | Longitude of the event location. |
| `name` | String | Name of the event. |
| `website` | String | Website URL for the event. |
| `starts_at` | ISO 8601 datetime | Start time of the event. |
| `ends_at` | ISO 8601 datetime or null | End time of the event, if it has an end time. |

#### Examples:

##### Fetch All Known Future Events

```bash
curl --request GET https://api.btcmap.org/v4/events | jq
```

```json
[
  {
    "id": 1,
    "area_id": 1,
    "lat": 7.8812324,
    "lon": 98.3884695,
    "name": "Phuket Bitcoin Meetup",
    "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
    "starts_at": "2025-08-29T19:00:00+07:00",
    "ends_at": null
  },
  {
    "id": 2,
    "area_id": null,
    "lat": 35.10219193997288,
    "lon": 129.0373886381881,
    "name": "Sats N Facts Busan",
    "website": "https://satsnfacts.xyz/",
    "starts_at": "2025-12-05T00:00:00+09:00",
    "ends_at": "2025-12-07T23:59:59+09:00"
  },
  {
    "id": 3,
    "area_id": 2,
    "lat": 18.782225261011515,
    "lon": 98.99429178234963,
    "name": "Weekly Bitcoin Mixer",
    "website": "https://www.meetup.com/bitcoinsinchiangmai/",
    "starts_at": "2025-08-07T19:00:00+07:00",
    "ends_at": null
  },
  {
    "id": 4,
    "area_id": 3,
    "lat": -8.643221369429375,
    "lon": 115.14280433620284,
    "name": "Bitcoin Indonesia Conference 2025",
    "website": "https://bitcoinindonesia.xyz/bitcoin-indonesia-conference-2025/",
    "starts_at": "2025-09-05T10:00:00+08:00",
    "ends_at": null
  }
]
```

### Get by ID

```
curl --request GET https://api.btcmap.org/v4/events/{id}
```

Retrieves a specific event by its ID.

#### Path Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `id` | Integer | `1` | - | **Required**. |

#### Examples

##### Get Specific Event

```
curl --request GET https://api.btcmap.org/v4/events/3 | jq
```

```json
{
  "id": 3,
  "area_id": 2,
  "lat": 18.782225261011515,
  "lon": 98.99429178234963,
  "name": "Weekly Bitcoin Mixer",
  "website": "https://www.meetup.com/bitcoinsinchiangmai/",
  "starts_at": "2025-08-07T19:00:00+07:00",
  "ends_at": null
}
```