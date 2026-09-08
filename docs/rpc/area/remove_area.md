# remove_area

## Description

Soft-deletes an area identified by its numeric `id` or `url_alias`. The row
is kept in the `area` table with `deleted_at` set to the current timestamp,
and the area will no longer show up in default queries. Use the REST API
with `include_deleted=true` to surface tombstones.

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

The soft-deleted area, identical in shape to [get_area](get_area.md), with
`deleted_at` populated:

```json
{
  "id": 123,
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok"
  },
  "created_at": "2024-01-15T12:34:56Z",
  "updated_at": "2024-06-30T08:00:00Z",
  "deleted_at": "2024-07-01T10:00:00Z"
}
```

## Allowed Roles

- Root
- Admin

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"remove_area","params":{"id":"bangkok"},"id":1}' \
  https://api.btcmap.org/rpc
```
