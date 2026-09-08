# get_wallets

## Description

Lists all known wallets. Each wallet has its own row in the `wallet` table; its `xpub` is scanned by the background refresher every 5 minutes against the highest-priority reachable `electrum_server`, and the resulting balance plus recent transactions are stored back on the row in `cached_balance_sats`, `cached_tx`, and `cached_at`. Soft-deleted rows are hidden unless `include_deleted` is set to `true`.

## Params

```json
{
  "include_deleted": false
}
```

- `include_deleted` (optional, default `false`): when `true`, soft-deleted rows are included in the result with a non-null `deleted_at` timestamp.

## Result Format

```json
[
  {
    "id": 1,
    "name": "donations",
    "xpub": "xpub...",
    "cached_balance_sats": 12345,
    "cached_tx": [
      {
        "id": "f418...",
        "received": 500,
        "sent": 300,
        "delta": 200
      }
    ],
    "cached_at": "2026-07-24T12:00:00.000Z",
    "created_at": "2026-07-24T11:00:00.000Z",
    "updated_at": "2026-07-24T12:00:00.000Z",
    "deleted_at": null
  }
]
```

## Fields

- `id`: Wallet id, use it for `update_wallet` and `remove_wallet`.
- `name`: Human-readable label. Unique across the table.
- `xpub`: Extended public key (BIP-32). Supports the standard `xpub`/`tpub` (legacy P2PKH), `ypub`/`upub` (nested segwit P2SH-P2WPKH), `zpub`/`vpub` (native segwit P2WPKH) prefixes, and the `taproot:` prefix for P2TR derivation.
- `cached_balance_sats`: Most recent balance, in satoshis, observed by the background refresher. `0` until the first refresh completes.
- `cached_tx`: Up to 10 most recent transactions affecting the wallet, each with `id` (txid), `received` (sats), `sent` (sats), and `delta` (received − sent). Empty until the first refresh completes.
- `cached_at`: Timestamp of the most recent refresh. `null` until the first refresh completes.
- `created_at`, `updated_at`: Standard audit timestamps.
- `deleted_at`: Soft-delete timestamp, `null` for active rows.

## Allowed Roles

- root
- admin

## Examples

### btcmap-cli

```bash
btcmap-cli wallet list
btcmap-cli wallet list --include-deleted
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_wallets","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
