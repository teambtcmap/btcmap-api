# Places REST API (v4)

This document describes the endpoints for interacting with places in REST API v4.

## Available Endpoints

- [Get Batch](#get-list)
- [Get by ID](#get-by-id)
- [Get Comments by Place ID](#get-comments)

### Get Bach

```bash
curl --request GET https://api.btcmap.org/v4/places
```

Retrieves a list of places. You can limit the output and apply a few useful filters.

#### Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `fields` | String | `id,name,icon` | `id` | A comma-separated list of requested fields. |
| `updated_since` | ISO 8601 datetime | `2025-01-01T00:00:00Z` | `1970-01-01T00:00:00Z` | Filter places updated since this time. |
| `include_deleted` | Boolean | `true` | `false` | Whether to include deleted places. |
| `limit` | Integer | `5` | - | Limit the number of places returned. |

##### Field Selection

The `fields` parameter allows you to request specific fields to be included in the response, which can improve performance for large requests.

Available fields include:


| Name | Type | Example | Description |
|------|------|---------|-------------|
| lat | Number | 53.2689435 | Place latitude. |
| lon | Number | 9.8538715 | Place longitude. |
| icon | String | cafe | [Material Icons](https://fonts.google.com/icons) identifier. |
| name | String | Der Schafstall | Place Name. Defaults to English, if available. |
| address | String | 5, Nowhere St. | Place address, if known. |
| opening_hours | String | Mo-Fr 08:00-12:00 | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key:opening_hours) for detailed format spec. |
| comments | Number | 2 | Number of comments. The comments themselves can be fetched via [Get Comments by Place ID](#get-comments) |
| created_at | ISO 8601 datetime | 2025-01-01T00:00:00Z | Returns a date when BTC Map started tracking that place. |
| updated_at | ISO 8601 datetime | 2025-01-01T00:00:00Z | Last change timestamp. Can be used for incremental sync. |
| deleted_at | ISO 8601 datetime | 2025-01-01T00:00:00Z | BTC Map API can return deleted places on request, to help client apps purge their caches. |
| verified_at | ISO 8601 date | 2025-02-03 | Last verification date. Recently verified places are more reliable so you might express it somehow in your app. You can also filter out places which haven't been verified for quite some time. |
| osm_id | String | node:1234 | OSM identifier, when available. |
| osm_url | URL | https://www.openstreetmap.org/node/12098197068 | OSM URL, when available. |
| phone | String | +60652249252 | Phone number associated with this POI. |
| website | URL | https://www.openstreetmap.org | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key%3Awebsite) for more details on expected format. |
| twitter | URL **OR** username | Satoshi | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key:contact:twitter) for more details on expected format. |
| facebook | URL **OR** username | Satoshi | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key:contact:facebook) for more details on expected format. |
| instagram | URL **OR** username | Satoshi | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key:contact:instagram) for more details on expected format. |
| line | URL **OR** username | Satoshi | Check [OSM Wiki](https://wiki.openstreetmap.org/wiki/Key:contact:line) for more details on expected format. |
| email | email | foo@bar.com | Email address that can be used to contact a place. |
| boosted_until | ISO 8601 datetime | 2025-01-01T00:00:00Z | This property indicates that a place is currently boosted, which is a good quality signal and you can display such places differently. |
| comments | Number | 2 | Place comment count. |

#### Examples:

##### Fetch All Active Places With Location and Name

```bash
curl --request GET https://api.btcmap.org/v4/places?fields=id,lat,lon,name | jq
```

```json
[
  {
    "id": 4829,
    "lat": 53.2689435,
    "lon": 9.8538715,
    "name": "Der Schafstall"
  },
  {
    "id": 5657,
    "lat": 47.049463,
    "lon": 8.3088867,
    "name": "das weisse schaf"
  },
  {
    "id": 12849,
    "lat": 16.597969,
    "lon": -22.9057133,
    "name": "Ocean Café Hotel"
  }
]
```

### Get by ID

```
curl --request GET https://api.btcmap.org/v4/places/{id}
```

Retrieves a specific place by its ID. It supports both BTC Map numerical IDs and OSM IDs (`element_type:id`).

#### Path Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `id` | String | `5` or `node:28` | - | **Required**. |
| `fields` | String | `id,name,icon` | `id` | A comma-separated list of requested fields. See [Field Selection](#field-selection) for a full list of available fields. |

#### Examples

##### Get Place Contact Details

```
curl --request GET https://api.btcmap.org/v4/places/5005?fields=id,name,phone,website | jq
```

```json
{
  "id": 5005,
  "name": "Casanova",
  "phone": "+41 562100084",
  "website": "https://www.casanovabaden.ch"
}
```

### Get Comments by Place ID

This is equivalent of filtering the `/place-comments` endpoint by `place_id`.

#### Examples

##### Get Comments for a Local Bar

```bash
curl --request GET https://api.btcmap.org/v4/places/22923/comments | jq
```

```json
[
  {
    "id": 1044,
    "text": "Best burgers in Phuket! Paid in sats",
    "created_at": "2025-01-06T15:14:03.8Z"
  },
  {
    "id": 1084,
    "text": "Visited and paid in sats",
    "created_at": "2025-01-12T11:03:50.83Z"
  },
  {
    "id": 1184,
    "text": "They have a nice neon Bitcoin sign",
    "created_at": "2025-02-21T05:07:06.379Z"
  }
]
```
