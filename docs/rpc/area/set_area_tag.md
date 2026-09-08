# set_area_tag

## Description

Sets or updates a single tag on an area identified by its numeric `id` or
`url_alias`. Updates are merged into the existing tags map; tags that are not
named in the request are left untouched.

Updating the `geo_json` tag rebuilds the area-to-element mapping for the
affected region, which can be expensive on large areas.

## Params

```json
{
  "id": "bangkok",
  "name": "icon:square",
  "value": "https://static.btcmap.org/images/areas/123.png"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Numeric area id or `url_alias` tag |
| `name` | string | Tag name to set |
| `value` | any | Tag value (must be valid JSON) |

## Result Format

The updated area, identical in shape to [get_area](get_area.md):

```json
{
  "id": 123,
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok",
    "icon:square": "https://static.btcmap.org/images/areas/123.png"
  },
  "created_at": "2024-01-15T12:34:56Z",
  "updated_at": "2024-06-30T08:00:00Z",
  "deleted_at": null
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli set-area-tag bangkok icon:square 'https://static.btcmap.org/images/areas/123.png'
```

The CLI also accepts a numeric id:

```bash
btcmap-cli set-area-tag 123 icon:square 'https://static.btcmap.org/images/areas/123.png'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"set_area_tag","params":{"id":"bangkok","name":"icon:square","value":"https://static.btcmap.org/images/areas/123.png"},"id":1}' \
  https://api.btcmap.org/rpc
```
