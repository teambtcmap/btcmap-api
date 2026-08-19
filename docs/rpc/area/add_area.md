# add_area

## Description

Creates a new geographic area. The area's geometry is captured by a `geo_json`
tag (any GeoJSON object — typically a `Polygon`), and every existing element
that falls inside that geometry is mapped to the new area as part of the same
transaction.

Two tags are required:

- `url_alias` — unique string identifier (e.g. `bangkok`). Once set it cannot
  be changed or removed.
- `geo_json` — GeoJSON geometry defining the area's boundary.

Common optional tags include `name` (display name) and `type` (`country`,
`community`, `neighborhood`, etc.). `type` is also used by the
[trending methods](get_trending_countries.md) to filter areas.

## Restrictions

Users with a non-empty geofence are blocked from creating new areas. If an
area manager has been assigned a geofence, they must ask an admin to add new
areas on their behalf. The handler rejects the call with the message
`Cannot add new areas when your geofence is set (allowed areas: {...})`.

## Params

```json
{
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok",
    "type": "community",
    "geo_json": {
      "type": "Polygon",
      "coordinates": [
        [
          [100.49, 13.75],
          [100.49, 13.78],
          [100.52, 13.78],
          [100.52, 13.75],
          [100.49, 13.75]
        ]
      ]
    }
  }
}
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tags` | object | yes | Map of area tags. Must contain `url_alias` and `geo_json`. |

## Result Format

```json
{
  "id": 123,
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok",
    "type": "community",
    "geo_json": {
      "type": "Polygon",
      "coordinates": [
        [
          [100.49, 13.75],
          [100.49, 13.78],
          [100.52, 13.78],
          [100.52, 13.75],
          [100.49, 13.75]
        ]
      ]
    }
  },
  "created_at": "2024-01-15T12:34:56Z",
  "updated_at": "2024-01-15T12:34:56Z",
  "deleted_at": null
}
```

## Allowed Roles

- Root
- Admin
- AreaManager (only when the caller has no geofence set)

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"add_area","params":{"tags":{"url_alias":"bangkok","name":"Bangkok","type":"community","geo_json":{"type":"Polygon","coordinates":[[[100.49,13.75],[100.49,13.78],[100.52,13.78],[100.52,13.75],[100.49,13.75]]]}}},"id":1}' \
  https://api.btcmap.org/rpc
```
