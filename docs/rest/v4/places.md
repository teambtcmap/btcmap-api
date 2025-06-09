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
| `include_deleted` | Boolean | `true` | `false` | Whether to include deleted elements. |
| `limit` | Integer | `5` | - | Limit the number of places returned. |

##### Field Selection

The `fields` parameter allows you to request specific fields to be included in the response, which can improve performance for large requests.

Available fields include:

```
lat // Place latitude
lon // Place longitude
icon // Place icon
name // Place name
comments // Number of comments
```

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
| `fields` | String | `id,name,icon` | `id` | A comma-separated list of requested fields. |

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
