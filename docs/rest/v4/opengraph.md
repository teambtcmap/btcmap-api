# OpenGraph REST API (v4)

Rendered map card images for OpenStreetMap elements. Use them as `og:image`
(or any other preview image) when linking to an OSM element that is not
necessarily a BTC Map place — an arbitrary shop, a park, a train station.

The element is looked up live in [Overpass](https://overpass-api.de/) so ways
and relations resolve to their centre without the client having to know the
geometry. The resulting card is a 600×315 JPEG: an OpenStreetMap raster map at
zoom 17 with the element's marker in the centre.

Cards are cached server-side for seven days, keyed by OSM type and ID. The
first request for an element renders the card, which is slower and depends on
Overpass and the tile server; because the public Overpass instance is shared,
that request can occasionally take tens of seconds. Later requests are served
from the cache until the entry expires, so ordinary web traffic never waits on
Overpass.

## Available Endpoints

- [Get OSM Element Card](#get-osm-element-card)

### Get OSM Element Card

```bash
curl -o card.jpg \
  'https://api.btcmap.org/v4/opengraph/osm-element/way:346185092'
```

This endpoint is **public** — no authentication header is required.

#### Path Parameters

| Parameter | Type   | Example               | Description                                                                                                              |
|-----------|--------|-----------------------|--------------------------------------------------------------------------------------------------------------------------|
| `id`      | String | `way:346185092`       | **Required**. An OSM reference in the form `<type>:<id>`, where `<type>` is `node`, `way` or `relation` and `<id>` is a positive integer. |

The type has to be sent explicitly: `123` and `node:123` are not treated the
same. This endpoint always resolves the element live from OSM, so it works for
any element, whether or not it is a BTC Map place. The separate
`/og/element/{id}` endpoint serves cards for BTC Map places only, from stored
BTC Map data.

#### Response

```http
HTTP/1.1 200 OK
Content-Type: image/jpeg
```

A 600×315 JPEG of the element's surroundings with the element marked at the
centre. Ways and relations are centred on the middle of their bounding box.
Render it as an image directly:

```html
<img src="https://api.btcmap.org/v4/opengraph/osm-element/relation:62422"
     width="600" height="315" alt="Map of the element">
```

#### Caching

Each rendered card is stored for seven days. Within that window the image is
served from the cache without contacting Overpass. Requesting the same element
again after the window re-renders it, so edits in OSM eventually show up.
Expired entries are pruned when a new card is rendered.

When Overpass or the tile server is unreachable, images already in the cache
are still served; only uncached elements fail with `502`.

#### Errors

Unlike the JSON endpoints in v4, this endpoint returns plain-text bodies for
4xx and 5xx responses.

| Status | Body                                         | Meaning                                                                                     |
|--------|----------------------------------------------|---------------------------------------------------------------------------------------------|
| 400    | `Expected node:ID, way:ID or relation:ID`    | `id` is not a `<type>:<id>` reference with a known type and a positive integer ID.           |
| 404    | `OSM element not found`                      | OSM has no element with that type and ID (including elements that were deleted).             |
| 502    | `Failed to fetch OSM element`                | Overpass could not be reached or returned an error, or the card could not be rendered.       |

A `502` does not poison the cache: nothing is stored, so retrying later renders
the card normally.
