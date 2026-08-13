# get_area

## Description

Retrieves a single area by its numeric `id` or its `url_alias` tag.

## Params

```json
{
  "id": "bangkok"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Numeric area id or `url_alias` tag |

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
btcmap-cli get-area bangkok
btcmap-cli get-area 123
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_area","params":{"id":"bangkok"},"id":1}' \
  https://api.btcmap.org/rpc
```
