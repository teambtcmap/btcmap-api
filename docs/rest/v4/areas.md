# Areas REST API (v4)

This document describes the endpoints for interacting with areas in REST API v4.

## Available Endpoints

- [Get Saved Areas](#get-saved-areas)
- [Set Saved Areas](#set-saved-areas)
- [Add Saved Area](#add-saved-area)
- [Delete Saved Area](#delete-saved-area)
- [Get Area Image](#get-area-image)
- [Get Events by Area](events.md#get-events-by-area)

### Get Saved Areas

Returns the authenticated user's saved areas.

```bash
curl https://api.btcmap.org/v4/areas/saved
```

**Requires authentication.** See [Users API](users.md) for details.

#### Examples

```json
[
  {
    "id": 123,
    "name": "Grand Paris",
    "type": "community",
    "url_alias": "grand-paris",
    "icon": "https://static.btcmap.org/images/communities/grand-paris.jpg",
    "website_url": "https://btcmap.org/community/grand-paris"
  }
]
```

### Set Saved Areas

Replaces the authenticated user's saved areas list.

```bash
curl -X PUT https://api.btcmap.org/v4/areas/saved \
  -H "Authorization: Bearer {token}" \
  -d '[123, 456, 789]'
```

**Requires authentication.** See [Users API](users.md) for details.

#### Request Body

| Type | Example | Description |
|------|---------|-------------|
| Array of Numbers | `[123, 456]` | Array of area IDs to save. |

#### Response

Returns the updated list of saved area IDs.

```json
[123, 456, 789]
```

### Add Saved Area

Adds a single area to the authenticated user's saved areas.

```bash
curl -X POST https://api.btcmap.org/v4/areas/saved \
  -H "Authorization: Bearer {token}" \
  -d 123
```

**Requires authentication.** See [Users API](users.md) for details.

#### Request Body

| Type | Example | Description |
|------|---------|-------------|
| Number | `123` | Area ID to add. |

#### Response

Returns the updated list of saved area IDs. If the area is already saved, the list is unchanged.

```json
[123, 456, 789]
```

### Delete Saved Area

Removes a single area from the authenticated user's saved areas.

```bash
curl -X DELETE https://api.btcmap.org/v4/areas/saved/123
```

**Requires authentication.** See [Users API](users.md) for details.

#### Path Parameters

| Parameter | Type | Example | Description |
|-----------|------|---------|-------------|
| `id` | Number | `123` | **Required**. Area ID to remove. |

#### Response

Returns the updated list of saved area IDs.

```json
[456, 789]
```

### Search

Search for areas containing a specific geographic coordinate. This is useful for finding which areas (countries, communities) a particular location belongs to.

```bash
curl https://api.btcmap.org/v4/areas
```

#### Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `lat` | Number | `48.8566` | - | **Required**. Latitude (-90 to 90). |
| `lon` | Number | `2.3522` | - | **Required**. Longitude (-180 to 180). |

#### Examples

##### Search for areas containing Paris, France

```bash
curl 'https://api.btcmap.org/v4/areas?lat=48.8566&lon=2.3522'
```

```json
[
  {
    "id": 123,
    "name": "Grand Paris",
    "type": "community",
    "url_alias": "grand-paris",
    "icon": "https://static.btcmap.org/images/communities/grand-paris.jpg",
    "website_url": "https://btcmap.org/community/grand-paris"
  }
]
```

##### Search for areas containing Phuket, Thailand

```bash
curl 'https://api.btcmap.org/v4/areas?lat=7.9&lon=98.3'
```

```json
[
  {
    "id": 671,
    "name": "Phuket Bitcoin",
    "type": "community",
    "url_alias": "bitcoin-powerhouse",
    "icon": "https://static.btcmap.org/images/areas/671_square.jpg",
    "website_url": "https://btcmap.org/community/bitcoin-powerhouse",
    "upcoming_events": [
      {
        "id": 122,
        "lat": 7.899971135374869,
        "lon": 98.36758749588596,
        "name": "Central Bitcoin Meetup",
        "website": "https://www.meetup.com/phuket-bitcoin-meetup/",
        "starts_at": "2026-09-25T19:00:00Z",
        "ends_at": "2026-09-25T22:00:00Z"
      }
    ]
  }
]
```

#### Response Fields

| Name | Type | Example | Description |
|------|------|---------|-------------|
| `id` | Number | `123` | BTC Map Area ID. |
| `name` | String | `Paris` | Area name. |
| `type` | String | `cities` | Area type (country, community). |
| `url_alias` | String | `paris` | URL-friendly identifier for the area. |
| `icon` | String or null | `https://static.btcmap.org/images/areas/123.png` | Square icon URL for the area, if set. |
| `website_url` | String | `https://btcmap.org/country/th` | URL to the BTC Map page for this area. |
| `upcoming_events` | Array of Event | `[]` | Upcoming events whose location falls inside this area's geometry. Only populated when `lat` and `lon` are provided; events without `starts_at` are excluded. See [Event](#event-object) below. |

#### Event Object

Each entry in `upcoming_events` matches the [Events API](events.md) response shape.

| Name | Type | Example | Description |
|------|------|---------|-------------|
| `id` | Number | `122` | Event ID. |
| `lat` | Number | `7.9` | Event latitude. |
| `lon` | Number | `98.3` | Event longitude. |
| `name` | String | `Central Bitcoin Meetup` | Event name. |
| `website` | String | `https://example.com` | Event website. |
| `starts_at` | String (RFC 3339) | `2026-09-25T19:00:00Z` | Event start time. |
| `ends_at` | String (RFC 3339), omitted when absent | `2026-09-25T22:00:00Z` | Event end time, if set. |

The endpoint has two mutually exclusive modes, selected by the query parameters:

- **Search mode** (the default): pass any of `lat`, `lon` or `type` and get the
  `AreaSearchResult` shape documented above.
- **Sync mode**: pass any of `fields`, `updated_since`, `limit` or
  `include_deleted` to page through the area change log.

Mixing a search parameter with a sync parameter returns `400 invalid_input`:

```bash
curl 'https://api.btcmap.org/v4/areas?lat=48.8566&lon=2.3522&fields=id'
```

### Sync Areas

Incrementally fetch area changes for an offline cache. Raw `tags` are never
returned; geometry is available through `bbox` (compact) or the full `geo_json`
polygon, which is only sent when explicitly requested in `fields`.

```bash
curl 'https://api.btcmap.org/v4/areas?fields=id,name,type,url_alias,icon,bbox,updated_at
  &updated_since=2025-06-11T00:00:00Z
  &include_deleted=true'
```

#### Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `fields` | String | `id,name,updated_at` | `id` | Comma-separated field projection. Only these fields are returned, plus `id`. See [Sync Fields](#sync-fields). |
| `updated_since` | String (RFC 3339) | `2025-06-11T00:00:00Z` | `1970-01-01T00:00:00Z` | Return areas whose `updated_at` is strictly after this instant. Omit it for a full snapshot. |
| `limit` | Number | `1000` | unlimited | Maximum number of areas to return. |
| `include_deleted` | Boolean | `true` | `false` | Include soft-deleted areas (tombstones). Implied when `deleted_at` is in `fields`. |
| `lang` | String | `ru` | base tag | Two-letter language code used to localize `name` and `description`. |

#### Sync Fields

Unknown field names are ignored. `id` is always returned.

| Field | Type | Description |
|-------|------|-------------|
| `id` | Number | BTC Map Area ID. |
| `name` | String | Area name, localized with `lang` when its `name:<lang>` tag exists. |
| `type` | String | Area type (country, community, etc.). |
| `url_alias` | String | URL-friendly identifier for the area. |
| `icon` | String | Square icon URL (`icon:square`), omitted when unset. |
| `icon_wide` | String | Wide icon URL (`icon:wide`), omitted when unset. |
| `website_url` | String | URL to the BTC Map page for this area. |
| `description` | String | Area description, localized with `lang`. |
| `bbox` | Array of 4 numbers | `[west, south, east, north]`, omitted when the area has no bbox of its own. |
| `geo_json` | Object | Full GeoJSON geometry/feature/feature collection, exactly as stored. Omitted when the area has no geometry. Large; request it only when the client needs the polygon itself. |
| `created_at` | String (RFC 3339) | Creation timestamp. |
| `updated_at` | String (RFC 3339) | Last update timestamp; use as the next `updated_since` cursor. |
| `deleted_at` | String (RFC 3339) | Tombstone timestamp. Present only for deleted areas when tombstones are included. |

#### Examples

A sync request for the fields a map needs:

```bash
curl 'https://api.btcmap.org/v4/areas?fields=id,name,type,icon,bbox&updated_since=2025-06-11T00:00:00Z'
```

```json
[
  {
    "id": 123,
    "name": "Grand Paris",
    "type": "community",
    "icon": "https://static.btcmap.org/images/communities/grand-paris.jpg",
    "bbox": [2.22, 48.81, 2.47, 48.91]
  }
]
```

An empty `fields` returns only `id`, which is enough to detect new and changed
areas:

```bash
curl 'https://api.btcmap.org/v4/areas?updated_since=2025-06-11T00:00:00Z'
```

```json
[{ "id": 123 }]
```

### Get Area

```
curl https://api.btcmap.org/v4/areas/{id}
```

Retrieves a specific area by its ID or alias (url slug).

#### Path Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `id` | String | `123` or `grand-paris` | - | **Required**. Area ID (numeric) or alias (url slug). |

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `lang` | String | `ru` | - | Two-letter language code. Localized `name:<lang>`/`description:<lang>` tags are preferred, falling back to `name:en`/`description:en` and then the base `name`/`description` tag. |

#### Examples

##### Get Area by ID

```bash
curl 'https://api.btcmap.org/v4/areas/123'
```

```json
{
  "id": 123,
  "name": "Grand Paris",
  "type": "community",
  "url_alias": "grand-paris",
  "icon": "https://static.btcmap.org/images/communities/grand-paris.jpg",
  "website_url": "https://btcmap.org/community/grand-paris",
  "description": "Grand Paris area covering the greater Paris metropolitan region."
}
```

##### Get Area by Alias

```bash
curl 'https://api.btcmap.org/v4/areas/grand-paris'
```

```json
{
  "id": 123,
  "name": "Grand Paris",
  "type": "community",
  "url_alias": "grand-paris",
  "icon": "https://static.btcmap.org/images/communities/grand-paris.jpg",
  "website_url": "https://btcmap.org/community/grand-paris",
  "description": "Grand Paris area covering the greater Paris metropolitan region."
}
```

#### Response Fields

| Name | Type | Example | Description |
|------|------|---------|-------------|
| `id` | Number | `123` | BTC Map Area ID. |
| `name` | String | `Grand Paris` | Area name. |
| `type` | String | `community` | Area type (country, region, community, etc.). |
| `url_alias` | String | `grand-paris` | URL-friendly identifier for the area. |
| `icon` | String or null | `https://static.btcmap.org/images/communities/grand-paris.jpg` | Square icon URL for the area, if set. |
| `website_url` | String | `https://btcmap.org/community/grand-paris` | URL to the BTC Map page for this area. |
| `description` | String | `Grand Paris area...` | Area description, if available. |

### Get Area Image

Supports on-the-fly raster resizing so clients can request exactly the
dimensions they need without downloading a multi-megabyte master asset.

```bash
curl -o area.png 'https://api.btcmap.org/v4/areas/grand-paris/image?type=square'
```

#### Path Parameters

| Parameter | Type | Example | Description |
|-----------|------|---------|-------------|
| `id` | String | `123` or `grand-paris` | **Required**. Area ID (numeric) or alias (url slug). |

#### Query Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `type` | String | `square` | - | **Required**. Which icon variant to fetch (e.g. `square`, `wide`). |
| `w` | Integer | `256` | source width | Maximum output width in pixels. See [Resizing](#resizing) below. |
| `h` | Integer | `256` | source height | Maximum output height in pixels. See [Resizing](#resizing) below. |

`w` and `h` must each be greater than `0` when provided; a value of `0`
returns `400 invalid_input`.

#### Resizing

Both `w` and `h` are optional. They are interpreted as maximum bounds — the
endpoint never upscales a smaller image.

- Both omitted → original bytes returned unchanged.
- Only `w` provided → scales `w` to the requested value, `h` derived from the
  source aspect ratio. Returns the original if the source is already narrower.
- Only `h` provided → mirror of the above.
- Both provided → fits the source into the `w`×`h` box, preserving the aspect
  ratio (smaller of the two ratios wins). If the source already fits, it is
  returned unchanged.

When the source and target dimensions match, the original bytes are returned
verbatim — no re-encoding is performed.

Only raster formats that can be both decoded and re-encoded are resized
(PNG, JPEG, WebP). Other formats are returned as-is:

- **SVG** (vector) → always returned as-is regardless of `w`/`h`, since
  raster resizing would defeat the point of vector data.
- **BMP or unknown** → returned as-is; the server cannot re-encode them.

The response `Content-Type` is set from the stored bytes (e.g. `image/png`,
`image/jpeg`, `image/svg+xml`), so clients can pipe the body straight to
disk or an `<img>` tag.

#### Examples

##### Fetch an area icon at its native size

```bash
curl -o square.png 'https://api.btcmap.org/v4/areas/grand-paris/image?type=square'
```

##### Resize to a thumbnail

```bash
curl -o thumb.png 'https://api.btcmap.org/v4/areas/grand-paris/image?type=square&w=128&h=128'
```

This fits the source into a 128×128 box while preserving the aspect ratio.

##### Resize by width only

```bash
curl -o banner.png 'https://api.btcmap.org/v4/areas/grand-paris/image?type=wide&w=600'
```

The height is derived from the source aspect ratio; the image is never
stretched vertically.

##### Upsize requests return the original

```bash
curl -o big.png 'https://api.btcmap.org/v4/areas/grand-paris/image?type=square&w=4000&h=4000'
```

If the stored image is smaller than 4000×4000, the original bytes are
returned unchanged.
