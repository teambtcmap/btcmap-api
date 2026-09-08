# update_wallet

## Description

Updates one or more fields of an existing wallet. Only the fields you pass are changed; omitted fields keep their current values. Names must remain unique across the table.

## Params

```json
{
  "id": 1,
  "name": "renamed",
  "xpub": "xpub...",
  "deleted_at": null
}
```

- `id` (required): Wallet id returned by `add_wallet` or `get_wallets`.
- `name` (optional): New human-readable label. Must remain unique.
- `xpub` (optional): New extended public key. See [`get_wallets`](get_wallets.md) for the supported prefixes.
- `deleted_at` (optional, tri-state): controls the soft-delete flag.
  - omit the field → leave `deleted_at` unchanged.
  - `null` → clear the soft-delete (un-delete the row so it can be scanned again).
  - `"2024-01-01T00:00:00Z"` → set `deleted_at` to the given RFC 3339 timestamp.

## Result Format

```json
{
  "id": 1,
  "name": "renamed",
  "xpub": "xpub...",
  "cached_balance_sats": 12345,
  "cached_tx": [],
  "cached_at": "2026-07-24T12:00:00.000Z",
  "created_at": "2026-07-24T11:00:00.000Z",
  "updated_at": "2026-07-24T12:34:56.000Z",
  "deleted_at": null
}
```

## Allowed Roles

- root
- admin

## Errors

- The server rejects the call when the `id` does not match any row.
- The server rejects the call when the new `name` collides with another row.

## Examples

### btcmap-cli

```bash
btcmap-cli wallet update 1 --name renamed
btcmap-cli wallet update 1 --name renamed --xpub 'xpub...'
btcmap-cli wallet update 1 --deleted-at '2024-01-01T00:00:00Z'   # soft-delete
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_wallet","params":{"id":1,"name":"renamed"},"id":1}' \
  https://api.btcmap.org/rpc

# Un-delete a soft-deleted row by passing `deleted_at: null`.
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_wallet","params":{"id":1,"deleted_at":null},"id":1}' \
  https://api.btcmap.org/rpc
```
