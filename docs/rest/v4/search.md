# Search

```
curl 'https://api.btcmap.org/v4/search/?q=hamburg'
```

Searches areas and places in one call. Places match against **every OSM tag value**, so a
query for a city name finds the places whose address is in that city, and localized `name:*`
tags are searched for free. Areas match on their name and URL alias.

Every whitespace-separated word must match, though different words may match different tags:
`q=hamburg cafe` finds a place with `addr:city=Hamburg` and `cuisine=cafe`.

## Parameters

| Parameter     | Type   | Default | Description                                                             |
|---------------|--------|---------|-------------------------------------------------------------------------|
| `q`           | String | -       | **Required.** At least 3 characters. At most 8 words are considered.     |
| `lat`         | Number | -       | Optional. Breaks relevance ties by proximity. Must be paired with `lon`. |
| `lon`         | Number | -       | Optional. Must be paired with `lat`.                                     |
| `limit`       | Number | `20`    | Capped at 100.                                                           |
| `offset`      | Number | `0`     | Capped at 10000.                                                         |
| `type_filter` | String | -       | `area` or `place`. Omit to search both.                                  |

## Ordering

Results are ranked by an exact name match, then a name prefix match, then a name substring
match, then a match on any other tag. At equal rank, areas precede places, and — when `lat`
and `lon` are supplied — nearer places precede farther ones.

Supplying `lat` and `lon` matters more than it looks. A query like `hamburg` matches thousands
of places that no place is actually *named*, so they all share the lowest rank. Without a
location to break the tie, the `limit` selects among them by name length.

## Examples

### Find a city and the places in it

```bash
curl 'https://api.btcmap.org/v4/search/?q=hamburg&lat=53.55&lon=9.99&limit=2'
```

```json
{
  "results": [
    {
      "type": "area",
      "id": 123,
      "name": "Hamburg",
      "alias": "hamburg",
      "bbox": [9.73, 53.39, 10.32, 53.74]
    },
    {
      "type": "place",
      "id": 28779,
      "name": "Kaffeeklatsch",
      "lat": 53.5601,
      "lon": 9.9722,
      "icon": "local_cafe",
      "address": "Lange Reihe 12 Hamburg 20099",
      "created_at": "2025-09-17T08:22:03.855Z",
      "updated_at": "2025-09-18T13:12:31.723Z",
      "verified_at": "2025-09-18T00:00:00Z",
      "osm_id": "node:13149030952"
    }
  ],
  "total_count": 1284,
  "has_more": true,
  "query": "hamburg",
  "pagination": { "offset": 0, "limit": 2, "total": 1284 }
}
```

### Restrict to places

```bash
curl 'https://api.btcmap.org/v4/search/?q=hamburg&type_filter=place'
```

## Result types

Every result carries a `type` discriminator.

`type: "area"` — `id`, `name`, `alias` (omitted when unset), and `bbox` as
`[west, south, east, north]` (omitted when the area has no bounding box of its own).

`type: "place"` — the same object returned by [`/v4/places/search`](places.md#search): `id`,
`lat`, `lon`, `icon`, `name`, plus the optional `address`, `opening_hours`, `comments`,
`verified_at`, `osm_id`, `phone`, `website`, `localized_name` and friends.

## Notes

Matching is a case-insensitive substring test over ASCII. Diacritics are not folded, so
`q=cafe` does not match `Café`.

Because every tag value is searchable, a query for a value that most places carry — `q=yes`
matches every `payment:*=yes` tag — returns a very large `total_count`. Use `limit` and
`offset`.
