# remove_area_tag

## Description

Removes a single tag from an area identified by its numeric `id` or
`url_alias`. The `url_alias` and `geo_json` tags are protected and cannot be
removed — `url_alias` because it is the immutable handle for the area, and
`geo_json` because the area's geometry is required.

## Params

```json
{
  "id": "bangkok",
  "tag": "icon:square"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Numeric area id or `url_alias` tag |
| `tag` | string | Tag name to remove |

## Result Format

The updated area, identical in shape to [get_area](get_area.md):

```json
{
  "id": 123,
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok"
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
btcmap-cli remove-area-tag bangkok icon:square
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"remove_area_tag","params":{"id":"bangkok","tag":"icon:square"},"id":1}' \
  https://api.btcmap.org/rpc
```
