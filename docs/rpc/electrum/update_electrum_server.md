# update_electrum_server

## Description

Updates one or more fields of an existing electrum server. Only the fields you pass are changed; omitted fields keep their current values. URLs must remain unique across the table.

## Params

```json
{
  "id": 1,
  "name": "renamed",
  "priority": 50,
  "spki_pin": "",
  "deleted_at": null
}
```

- `id` (required): Server id returned by `add_electrum_server` or `get_electrum_servers`.
- `name` (optional): New human-readable label.
- `url` (optional): New connection URL.
- `priority` (optional): New priority.
- `spki_pin` (optional): New SPKI pin (`sha256:<64 hex chars>`) to pin a self-signed certificate. Pass an empty string to clear an existing pin and go back to standard CA validation.
- `deleted_at` (optional, tri-state): controls the soft-delete flag.
  - omit the field → leave `deleted_at` unchanged.
  - `null` → clear the soft-delete (un-delete the row so it can be used again).
  - `"2024-01-01T00:00:00Z"` → set `deleted_at` to the given RFC 3339 timestamp.

## Result Format

```json
{
  "id": 1,
  "name": "renamed",
  "url": "ssl://electrum.blockstream.info:50002",
  "priority": 50,
  "spki_pin": "",
  "created_at": "2026-07-24T12:00:00.000Z",
  "updated_at": "2026-07-24T12:34:56.000Z",
  "deleted_at": null
}
```

## Allowed Roles

- root
- admin

## Errors

- The server rejects the call when the `id` does not match any row.
- The server rejects the call when the new `url` collides with another active row.

## Examples

### btcmap-cli

```bash
btcmap-cli electrum-server update 1 --priority 50
btcmap-cli electrum-server update 1 --name renamed --priority 50
btcmap-cli electrum-server update 1 --spki-pin 'sha256:9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7'
btcmap-cli electrum-server update 1 --spki-pin ''   # clear pin
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_electrum_server","params":{"id":1,"priority":50},"id":1}' \
  https://api.btcmap.org/rpc

# Un-delete a soft-deleted row by passing `deleted_at: null`.
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_electrum_server","params":{"id":1,"deleted_at":null},"id":1}' \
  https://api.btcmap.org/rpc
```
