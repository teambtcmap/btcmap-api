# Places REST API (v4)

This document describes the endpoints for interacting with places in REST API v4.

## Available Endpoints

- [Chronological Sync](#chronological-sync)
- [Search](#search)
- [Fetch Place](#fetch-place)
- [Fetch Place Comments](#fetch-place-comments)

### Chronological Sync

Caching clients are advised to use this endpoint to sync all places and then request lightweight patches containing only the latest changes. By using the `updated_since` and `limit` parameters, a client can incrementally process the entire history until it reaches the tip, a process similar to Bitcoin's Initial Block Download (IBD) and progressive sync.

Bundling a recent snapshot with your app provides resilience against BTC Map server outages and offline functionality for users with poor or censored internet access.

```bash
curl https://api.btcmap.org/v4/places
```

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
| website | URL | https://www.openstreetmap.org | Raw [OSM tags](https://wiki.openstreetmap.org/wiki/Key%3Awebsite) are not guaranteed to be URLs. We force-convert them and filter out invalid URLs to simplify client code. |
| twitter | URL | https://x.com/barberosdelopez | Raw [OSM tags](https://wiki.openstreetmap.org/wiki/Key:contact:twitter) are not guaranteed to be URLs. We force-convert them and filter out invalid URLs to simplify client code. |
| facebook | URL | https://www.facebook.com/Gr33nPapaya | Raw [OSM tags](https://wiki.openstreetmap.org/wiki/Key:contact:facebook) are not guaranteed to be URLs. We force-convert them and filter out invalid URLs to simplify client code. |
| instagram | URL | https://www.instagram.com/vempromix23/ | Raw [OSM tags](https://wiki.openstreetmap.org/wiki/Key:contact:instagram) are not guaranteed to be URLs. We force-convert them and filter out invalid URLs to simplify client code. |
| line | URL | https://page.line.me/gcs8865c | Raw [OSM tags](https://wiki.openstreetmap.org/wiki/Key:contact:line) are not guaranteed to be URLs. We force-convert them and filter out invalid URLs to simplify client code. |
| email | email | foo@bar.com | Email address that can be used to contact a place. |
| boosted_until | ISO 8601 datetime | 2025-01-01T00:00:00Z | This property indicates that a place is currently boosted, which is a good quality signal and you can display such places differently. |
| required_app_url | URL | https://www.qerko.com | An additional app may be necessary at some locations to convert non-standard QR codes into the standard formats supported by most Bitcoin wallets. |

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

### Search

This method has two main use cases:

1. Client apps without cache, which need to fetch the places on demand for a small region (usually user map viewport). This endpoint is fairly optimized and you can call it every time user moves the map.
2. Client apps requiring server-side search. You can search places by area, name and also by payment provider.

#### Examples

##### Search Places by Payment Provider

Let's fetch all the places using [Coinos](https://coinos.io/), a popular Canadian Bitcoin-only payment provider and our sponsor.

```bash
curl 'https://api.btcmap.org/v4/places/search/?payment_provider=coinos' | jq
```

```json
[
  {
    "id": 28779,
    "lat": 49.1055648,
    "lon": -121.9641064,
    "icon": "menu_book",
    "name": "The Owl and The Cat Bookery"
  }
]
```

##### Search Places by Payment Provider and Name

Search filters can be mixed, so let's filter by both provider and merchant name:

```bash
curl 'https://api.btcmap.org/v4/places/search/?payment_provider=coinos&name=lounge' | jq
```

```json
[
  {
    "id": 19104,
    "lat": 49.281207,
    "lon": -123.0154316,
    "icon": "smoking_rooms",
    "name": "Bula Lounge"
  }
]
```

##### Search Places by Name

You need to provide at least 3 letters.

```bash
curl 'https://api.btcmap.org/v4/places/search/?name=thai' | jq
```

```json
[
  {
    "id": 21555,
    "lat": 16.6429003,
    "lon": 103.9031675,
    "icon": "restaurant",
    "name": "ก๋วยเตี๋ยวยกล้อ Thai Noodle"
  }
]
```

##### Search Places by Area

Let's fetch all the places in Manchester, UK.

```bash
curl 'https://api.btcmap.org/v4/places/search/?lat=53.48&lon=-2.24&radius_km=20' | jq
```

```json
[
  {
    "id": 2977,
    "lat": 53.448488,
    "lon": -2.2502728,
    "icon": "storefront",
    "name": "Manchester Appliance Repairs"
  }
]
```

### Fetch Place

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

### Fetch Place Comments

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
