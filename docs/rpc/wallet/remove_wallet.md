# remove_wallet

## Description

Soft-deletes a wallet so it stops being scanned by BTC Map API. The row is preserved with a non-null `deleted_at`.

## Params

```json
{
  "id": 1
}
```

- `id` (required): Wallet id returned by `add_wallet` or `get_wallets`.

## Result Format

```json
{
  "id": 1,
  "name": "donations",
  "xpub": "xpub...",
  "cached_balance_sats": 12345,
  "cached_tx": [],
  "cached_at": "2026-07-24T12:00:00.000Z",
  "created_at": "2026-07-24T11:00:00.000Z",
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
btcmap-cli wallet remove 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"remove_wallet","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```
