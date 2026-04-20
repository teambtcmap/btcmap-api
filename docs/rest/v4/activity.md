# Activity REST API (v4)

This document describes the endpoint for fetching a merged feed of
place-related activity in REST API v4.

## Available Endpoints

- [Get Activity Feed](#get-activity-feed)

### Get Activity Feed

```bash
curl --request GET https://api.btcmap.org/v4/activity
```

Returns a merged, time-sorted feed of recent place activity — creates,
updates, deletes, comments, and paid boosts — sorted newest-first. The
response is a flat array; each item carries a `type` discriminating the
kind of activity. Results can be scoped to one or more areas, one or
more places, or the combination of both.

This endpoint is **public** — no authentication header is required.

#### Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `days` | Integer | `7` | `1` | Lookback window in days. Must be `1 <= days <= 3650`. |
| `area` | String (ID or alias) | `germany` | - | Scope to a single area. Accepts the numeric area ID or the `url_alias`. Ignored when `areas` is provided. |
| `areas` | Comma-separated list of IDs / aliases | `germany,berlin` | - | Scope to multiple areas. Takes precedence over `area`. |
| `places` | Comma-separated list of integer place IDs | `38625,23143` | - | Scope to explicit places. Accepts at most 500 comma-separated values. |

When both `areas` and `places` are provided, the element set is the
**union** of elements belonging to any of the areas plus the explicit
place IDs. Duplicates are removed server-side, so saving a country
*and* a specific place inside that country will not produce duplicate
activity items.

#### Response Shape

```jsonc
[
  {
    "type": "place_added",          // place_added | place_updated | place_deleted | place_commented | place_boosted
    "place_id": 38625,
    "place_name": "Example Cafe",   // optional
    "osm_user_id": 12345,           // optional; present for place_added / place_updated / place_deleted
    "osm_user_name": "alice",       // optional; matches osm_user_id
    "osm_user_tip": "lightning:…",  // optional; parsed from the OSM user's description
    "comment": "great spot",        // optional; present for place_commented
    "duration_days": 30,            // optional; present for place_boosted
    "image": "https://api.btcmap.org/og/element/38625",
    "date": "2026-04-20T12:00:00Z"
  }
]
```

#### Examples

##### Global activity, last 24 hours

```bash
curl --request GET https://api.btcmap.org/v4/activity | jq
```

##### All activity in Germany over the last week

```bash
curl --request GET "https://api.btcmap.org/v4/activity?area=germany&days=7" | jq
```

##### Activity for a user's saved places and saved areas

```bash
curl --request GET \
  "https://api.btcmap.org/v4/activity?areas=1,2&places=38625,23143&days=30" | jq
```

#### Error Cases

| HTTP status | `code` | Condition |
|-------------|--------|-----------|
| 400 | `invalid_input` | `days` outside `[1, 3650]`. |
| 400 | `invalid_input` | `places` contains a non-integer value. |
| 400 | `invalid_input` | `places` contains more than 500 comma-separated values. |
| 404 | `not_found` | An ID or alias in `area` / `areas` does not resolve to an area. |
