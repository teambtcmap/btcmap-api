# Events REST API (v4)

This document describes the endpoints for interacting with events in REST API v4.

Upcoming events are also returned by the combined [`/v4/search`](search.md) endpoint,
which matches them by name alongside areas and places.

## Available Endpoints

- [Get Batch](#get-list)
- [Get by ID](#get-by-id)
- [Get Events by Area](#get-events-by-area)

### Get Batch

```bash
curl --request GET https://api.btcmap.org/v4/events
```

Retrieves a list of events. By default this is a full snapshot of all non-deleted
events with future start dates. Supplying `updated_since` switches to delta sync,
described below.

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `updated_since` | RFC 3339 datetime | `2025-01-01T00:00:00Z` | absent | Enables **delta sync**: returns every event with `updated_at` after this instant. When omitted, the legacy full snapshot is returned. |
| `limit` | Integer | `1000` | unlimited | Maximum number of events to return in delta mode. |
| `include_deleted` | Boolean | `true` | `false` | Include soft-deleted events so clients can apply tombstones. Only meaningful together with `updated_since`. |

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | Integer | Unique identifier for the event. |
| `lat` | Float | Latitude of the event location. |
| `lon` | Float | Longitude of the event location. |
| `name` | String | Name of the event. |
| `website` | String | Website URL for the event. |
| `starts_at` | ISO 8601 datetime | Start time of the event. |
| `ends_at` | ISO 8601 datetime (omitted when absent) | End time of the event, if it has an end time. |
| `updated_at` | ISO 8601 datetime (delta mode only) | When the event was last changed. Omitting `updated_since` leaves it out so existing clients see an unchanged payload. |
| `deleted_at` | ISO 8601 datetime (delta mode only, omitted when not deleted) | Soft-deletion time. Only returned with `updated_since` and `include_deleted=true`. |

#### Delta Sync

Delta mode is meant for clients that keep a local cache:

- The response is ordered by `updated_at`, then `id`, and each item carries
  `updated_at` so the client can advance its cursor.
- With `include_deleted=true`, soft-deleted events are returned with
  `deleted_at` set; the client should delete those rows locally. Non-deleted
  events omit `deleted_at`.
- Unlike the full snapshot, delta mode **does not** filter out events whose
  `starts_at` is in the past. A change log must still report edits and
  deletions of already-started events, so clients should filter or prune past
  events locally.

> **Cursor paging:** the cursor is a strict `updated_at` comparison. If a full
> page shares a single `updated_at` value, retry with a larger `limit`; the same
> strategy is used by [`/v4/places`](places.md) and
> [`/v4/place-comments`](place-comments.md).

#### Examples:

##### Fetch All Known Future Events

```bash
curl --request GET https://api.btcmap.org/v4/events | jq
```

```json
[
  {
    "id": 1,
    "lat": 7.8812324,
    "lon": 98.3884695,
    "name": "Phuket Bitcoin Meetup",
    "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
    "starts_at": "2025-08-29T19:00:00+07:00"
  },
  {
    "id": 2,
    "lat": 35.10219193997288,
    "lon": 129.0373886381881,
    "name": "Sats N Facts Busan",
    "website": "https://satsnfacts.xyz/",
    "starts_at": "2025-12-05T00:00:00+09:00",
    "ends_at": "2025-12-07T23:59:59+09:00"
  },
  {
    "id": 3,
    "lat": 18.782225261011515,
    "lon": 98.99429178234963,
    "name": "Weekly Bitcoin Mixer",
    "website": "https://www.meetup.com/bitcoinsinchiangmai/",
    "starts_at": "2025-08-07T19:00:00+07:00"
  },
  {
    "id": 4,
    "lat": -8.643221369429375,
    "lon": 115.14280433620284,
    "name": "Bitcoin Indonesia Conference 2025",
    "website": "https://bitcoinindonesia.xyz/bitcoin-indonesia-conference-2025/",
    "starts_at": "2025-09-05T10:00:00+08:00"
  }
]
```

##### Sync changes since a cursor

```bash
curl 'https://api.btcmap.org/v4/events?updated_since=2025-01-01T00:00:00Z&limit=1000&include_deleted=true' | jq
```

```json
[
  {
    "id": 5,
    "lat": 51.5074,
    "lon": -0.1278,
    "name": "London Bitcoin Meetup",
    "website": "https://example.com/london",
    "starts_at": "2025-02-01T18:00:00Z",
    "updated_at": "2025-01-15T09:30:00Z"
  },
  {
    "id": 6,
    "lat": 48.8566,
    "lon": 2.3522,
    "name": "Paris Bitcoin Meetup",
    "website": "https://example.com/paris",
    "starts_at": "2025-03-01T18:00:00Z",
    "updated_at": "2025-01-16T11:00:00Z",
    "deleted_at": "2025-01-16T11:00:00Z"
  }
]
```

The second item is a tombstone: delete it locally rather than displaying it.

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
  "lat": 18.782225261011515,
  "lon": 98.99429178234963,
  "name": "Weekly Bitcoin Mixer",
  "website": "https://www.meetup.com/bitcoinsinchiangmai/",
  "starts_at": "2025-08-07T19:00:00+07:00"
}
```

### Get Events by Area

```bash
curl 'https://api.btcmap.org/v4/areas/{id_or_alias}/events'
```

Returns all events whose `(lat, lon)` point lies inside the area's geometry.

The area's `bbox_*` columns are used as a cheap pre-filter at the SQL level;
each surviving candidate is then verified against the area's `geo_json`
geometries with a precise point-in-polygon check.

#### Path Parameters

| Parameter | Type | Example | Description |
|-----------|------|---------|-------------|
| `id_or_alias` | String | `123` or `phuket` | **Required**. Area ID (numeric) or alias (url slug). |

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `from` | RFC 3339 datetime | `2025-01-01T00:00:00Z` | now (UTC) | Only include events with `starts_at >= from`. Lower the value to include past events. |
| `to` | RFC 3339 datetime | `2025-12-31T23:59:59Z` | `2200-01-01T00:00:00Z` | Only include events with `starts_at <= to`. |

#### Examples

##### Future events for an area (default behavior)

```bash
curl 'https://api.btcmap.org/v4/areas/phuket/events'
```

```json
[
  {
    "id": 1,
    "lat": 7.8812324,
    "lon": 98.3884695,
    "name": "Phuket Bitcoin Meetup",
    "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
    "starts_at": "2025-08-29T19:00:00+07:00"
  }
]
```

##### Past and future events within a date window

```bash
curl 'https://api.btcmap.org/v4/areas/phuket/events?from=2020-01-01T00:00:00Z&to=2030-01-01T00:00:00Z'
```

##### Resolving an area by alias

```bash
curl 'https://api.btcmap.org/v4/areas/grand-paris/events?from=2025-01-01T00:00:00Z'
```

##### 404 for an unknown area

```bash
curl -i 'https://api.btcmap.org/v4/areas/does-not-exist/events'
# HTTP/1.1 404 Not Found
```

##### 400 for an unparseable date

```bash
curl -i 'https://api.btcmap.org/v4/areas/phuket/events?from=not-a-date'
# HTTP/1.1 400 Bad Request
```