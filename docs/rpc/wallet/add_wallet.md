# add_wallet

## Description

Adds a new wallet to the `wallet` table. Wallet names must be unique.

## Params

```json
{
  "name": "donations",
  "xpub": "xpub..."
}
```

- `name` (required): Human-readable label. Must be unique.
- `xpub` (required): Extended public key. See [`get_wallets`](get_wallets.md) for the supported prefixes.

## Result Format

```json
{
  "id": 1,
  "name": "donations",
  "xpub": "xpub...",
  "cached_balance_sats": 0,
  "cached_tx": [],
  "cached_at": null,
  "created_at": "2026-07-24T11:00:00.000Z",
  "updated_at": "2026-07-24T11:00:00.000Z",
  "deleted_at": null
}
```

## Allowed Roles

- root
- admin

## Errors

- The server rejects the call when `name` already exists for an active or soft-deleted row.
- Missing `name` or `xpub` is reported as a JSON-RPC parse error.

## Examples

### btcmap-cli

```bash
btcmap-cli wallet add \
  --name donations \
  --xpub 'xpub...'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"add_wallet","params":{"name":"donations","xpub":"xpub..."},"id":1}' \
  https://api.btcmap.org/rpc
```
