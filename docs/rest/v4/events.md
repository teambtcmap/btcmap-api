# Events REST API (v4)

This document describes the endpoints for interacting with events in REST API v4.

Upcoming events are also returned by the combined [`/v4/search`](search.md) endpoint,
which matches them by name alongside areas and places.

## Available Endpoints

- [Get Batch](#get-list)
- [Get by ID](#get-by-id)
- [Get Events by Area](#get-events-by-area)
- [Submit Event](#submit-event)
- [Change Event Status](#change-event-status)
- [Get My Submitted Events](#get-my-submitted-events)

### Get Batch

```bash
curl --request GET https://api.btcmap.org/v4/events
```

Retrieves a list of events. By default this is a full snapshot of all non-deleted
events with future start dates and `status=live`. Supplying `updated_since`
switches to delta sync, described below.

> **Review visibility:** pending and rejected submissions are hidden from every
> public read endpoint unless the caller explicitly opts in with the `status`
> query parameter. The default is therefore safe for older clients and shared
> caches; clients that render review state pass `status=all` (or a subset such
> as `status=pending,live`) to receive them. Note that because the default
> excludes non-live rows, a cached client stays unaware of events that never
> became live.

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `updated_since` | RFC 3339 datetime | `2025-01-01T00:00:00Z` | absent | Enables **delta sync**: returns every event with `updated_at` after this instant. When omitted, the legacy full snapshot is returned. |
| `limit` | Integer | `1000` | unlimited | Maximum number of events to return in delta mode. |
| `include_deleted` | Boolean | `true` | `false` | Include soft-deleted events so clients can apply tombstones. Only meaningful together with `updated_since`. |
| `status` | Comma-separated list or `all` | `all`, `pending,live` | `live` | Which review states to return. Omit for the public live-only view. |

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | Integer | Unique identifier for the event. |
| `lat` | Float | Latitude of the event location. |
| `lon` | Float | Longitude of the event location. |
| `name` | String | Name of the event. |
| `website` | String | Website URL for the event. |
| `status` | String | Review state: `pending`, `live` or `rejected`. All v4 event endpoints expose it, including batch, by-ID, by-area, search and area `upcoming_events`. New user submissions start as `pending`; privileged submissions are `live` immediately. |
| `submitted_by` | Object (omitted when unknown) | `{ "id": 123, "name": "satoshi" }` — `id` and `name` of the user who submitted the event. Present for events submitted through `POST /v4/events`; omitted for older events and RPC-created ones. |
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
    "status": "live",
    "starts_at": "2025-08-29T19:00:00+07:00"
  },
  {
    "id": 2,
    "lat": 35.10219193997288,
    "lon": 129.0373886381881,
    "name": "Sats N Facts Busan",
    "website": "https://satsnfacts.xyz/",
    "status": "live",
    "starts_at": "2025-12-05T00:00:00+09:00",
    "ends_at": "2025-12-07T23:59:59+09:00"
  },
  {
    "id": 3,
    "lat": 18.782225261011515,
    "lon": 98.99429178234963,
    "name": "Weekly Bitcoin Mixer",
    "website": "https://www.meetup.com/bitcoinsinchiangmai/",
    "status": "live",
    "starts_at": "2025-08-07T19:00:00+07:00"
  },
  {
    "id": 4,
    "lat": -8.643221369429375,
    "lon": 115.14280433620284,
    "name": "Bitcoin Indonesia Conference 2025",
    "website": "https://bitcoinindonesia.xyz/bitcoin-indonesia-conference-2025/",
    "status": "live",
    "starts_at": "2025-09-05T10:00:00+08:00"
  }
]
```

##### Sync changes since a cursor

```bash
curl 'https://api.btcmap.org/v4/events?updated_since=2025-01-01T00:00:00Z&limit=1000&include_deleted=true&status=all' | jq
```

```json
[
  {
    "id": 5,
    "lat": 51.5074,
    "lon": -0.1278,
    "name": "London Bitcoin Meetup",
    "website": "https://example.com/london",
    "status": "live",
    "starts_at": "2025-02-01T18:00:00Z",
    "updated_at": "2025-01-15T09:30:00Z"
  },
  {
    "id": 6,
    "lat": 48.8566,
    "lon": 2.3522,
    "name": "Paris Bitcoin Meetup",
    "website": "https://example.com/paris",
    "status": "rejected",
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

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `status` | Comma-separated list or `all` | `all` | `live` | Which review states may be returned. A non-live event reads as `404 Not Found` unless its status is included here. |

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
  "status": "live",
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
| `status` | Comma-separated list or `all` | `all`, `pending` | `live` | Which review states to return. Omit for the public live-only view. |

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
    "status": "live",
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

### Submit Event

```bash
curl --request POST https://api.btcmap.org/v4/events \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '{
    "lat": 7.9812,
    "lon": 98.3345,
    "name": "Phuket Bitcoin Meetup",
    "website": "https://example.com/phuket",
    "starts_at": "2026-09-25T19:00:00+07:00"
  }'
```

Submits a new event as the authenticated user. The initial `status` depends on the
caller's roles:

- `event_manager`, `admin` and `root` users submit **live** events. They are bound
  by their geofence: `(lat, lon)` must fall inside one of the areas in the user's
  geofence, unless the geofence is empty (global scope).
- Every other authenticated user submits a **pending** event, which an event
  manager, admin or root can later approve or reject.

The event is attributed to the caller, so it can be tracked afterwards through
[Get My Submitted Events](#get-my-submitted-events).

#### Request Body

| Field | Type | Example | Default | Description |
|-------|------|---------|---------|-------------|
| `lat` | Float | `7.9812` | - | **Required**. Latitude, between -90 and 90. |
| `lon` | Float | `98.3345` | - | **Required**. Longitude, between -180 and 180. |
| `name` | String | `Phuket Bitcoin Meetup` | - | **Required**. Non-empty. |
| `website` | String | `https://example.com` | - | **Required**. May be an empty string. |
| `starts_at` | String | `2026-09-25T19:00:00+07:00` | - | **Required**. RFC 3339 with an offset, or a floating local datetime that needs `timezone`. |
| `ends_at` | String | `2026-09-25T22:00:00+07:00` | absent | Optional end time. |
| `timezone` | String | `auto` or `Europe/Berlin` | absent | Required when a timestamp is floating (no offset). `auto` infers the zone from `lat`/`lon`. |
| `area_id` | Integer | `123` | absent | Optional area to associate with the event. |

#### Responses

| Status | Description |
|--------|-------------|
| `200` | Event created. The body is the full [event object](#response-fields), including its `status`. |
| `400` | Invalid coordinates, empty name, or an unparseable/missing timestamp. |
| `401` | Missing or invalid bearer token. |
| `403` | A privileged submitter tried to create an event outside their geofence. |

### Change Event Status

```bash
curl --request PUT https://api.btcmap.org/v4/events/123/status \
  --header 'Authorization: Bearer <token>' \
  --header 'Content-Type: application/json' \
  --data '{"status": "live"}'
```

Approves or rejects an event. Only `event_manager`, `admin` and `root` users may
call it, and the event location must be inside the caller's geofence (when set).

#### Path Parameters

| Parameter | Type | Example | Description |
|-----------|------|---------|-------------|
| `id` | Integer | `123` | **Required**. Event ID. |

#### Request Body

| Field | Type | Example | Description |
|-------|------|---------|-------------|
| `status` | String | `live` | **Required**. Either `live` or `rejected`. `pending` is rejected here because events are only *created* as pending. |

#### Responses

| Status | Description |
|--------|-------------|
| `200` | Status updated. The body is the full event object. |
| `400` | `status` is missing or is not `live`/`rejected`. |
| `401` | Missing or invalid bearer token. |
| `403` | The caller is not an event manager/admin/root, or the event is outside their geofence. |
| `404` | No event with the requested ID. |

### Get My Submitted Events

```bash
curl https://api.btcmap.org/v4/users/me/events \
  --header 'Authorization: Bearer <token>'
```

Lists every non-deleted event submitted by the authenticated user, newest first,
across all statuses (including `pending` and `rejected`) and regardless of whether
the event has already started. Use it to track the review outcome of submissions
made through [Submit Event](#submit-event). Requires a Bearer token.

#### Response

Returns an array of [event objects](#response-fields), each carrying its `status`.

| Status | Description |
|--------|-------------|
| `200` | Success - Returns the caller's events (possibly empty). |
| `401` | Missing or invalid bearer token. |
| `500` | Internal Server Error - Database error. |

##### Example Response (200 OK)

```json
[
  {
    "id": 174,
    "lat": 1.0,
    "lon": 2.0,
    "name": "My Pending Meetup",
    "website": "https://example.com",
    "status": "pending",
    "submitted_by": { "id": 123, "name": "satoshi" },
    "starts_at": "2026-11-01T18:00:00Z"
  },
  {
    "id": 168,
    "lat": 48.8566,
    "lon": 2.3522,
    "name": "My Rejected Meetup",
    "website": "https://example.com",
    "status": "rejected",
    "submitted_by": { "id": 123, "name": "satoshi" },
    "starts_at": "2026-09-01T18:00:00Z"
  }
]
```