# Areas REST API (v4)

This document describes the endpoints for interacting with areas in REST API v4.

## Available Endpoints

- [Search](#search)
- [Get Area](#get-area)

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
    "id": 456,
    "name": "Phuket Bitcoin Meetup",
    "type": "community",
    "url_alias": "bitcoin-powerhouse",
    "icon": "https://static.btcmap.org/images/areas/671.jpg",
    "website_url": "https://btcmap.org/community/bitcoin-powerhouse"
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

### Get Area

```
curl https://api.btcmap.org/v4/areas/{id}
```

Retrieves a specific area by its ID or alias (url slug).

#### Path Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `id` | String | `123` or `grand-paris` | - | **Required**. Area ID (numeric) or alias (url slug). |

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
