# remove_electrum_server

## Description

Soft-deletes an electrum server so it stops being used by BTC Map API. The row is preserved with a non-null `deleted_at`. The same URL can later be re-added with `add_electrum_server`.

## Params

```json
{
  "id": 1
}
```

- `id` (required): Server id returned by `add_electrum_server` or `get_electrum_servers`.

## Result Format

```json
{
  "id": 1,
  "name": "Blockstream",
  "url": "ssl://electrum.blockstream.info:50002",
  "priority": 0,
  "spki_pin": "",
  "created_at": "2026-07-24T12:00:00.000Z",
  "updated_at": "2026-07-24T12:34:56.000Z",
  "deleted_at": "2026-07-24T12:34:56.000Z"
}
```

## Allowed Roles

- root
- admin

## Errors

- The server rejects the call when the `id` does not match any row.

## Examples

### btcmap-cli

```bash
btcmap-cli electrum-server remove 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"remove_electrum_server","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```
