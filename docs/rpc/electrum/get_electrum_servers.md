# get_electrum_servers

## Description

Lists all known electrum servers. Servers are returned ordered by descending `priority`. Soft-deleted rows are hidden unless `include_deleted` is set to `true`.

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
    "name": "Internal 1",
    "url": "ssl://electrum-1.btcmap.org:50002",
    "priority": 100,
    "spki_pin": "sha256:9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7",
    "created_at": "2026-07-24T12:00:00.000Z",
    "updated_at": "2026-07-24T12:00:00.000Z",
    "deleted_at": null
  },
  {
    "id": 2,
    "name": "Blockstream",
    "url": "ssl://electrum.blockstream.info:50002",
    "priority": 0,
    "spki_pin": "",
    "created_at": "2026-07-24T12:00:00.000Z",
    "updated_at": "2026-07-24T12:00:00.000Z",
    "deleted_at": null
  }
]
```

## Fields

- `id`: Server id, use it for `update_electrum_server` and `remove_electrum_server`.
- `name`: Human-readable label.
- `url`: Connection URL passed to the electrum client. Supports encrypted `ssl://` and unencrypted `tcp://` URLs.
- `priority`: Higher values are tried first. Ties are broken by `id` ascending so the order is stable.
- `spki_pin`: SHA-256 hash of the server's SubjectPublicKeyInfo (`sha256:<64 hex chars>`), or empty when the server uses a public CA-signed certificate. When set, the wallet scan only accepts the connection if the server's certificate matches the pin, which is what you want for self-hosted servers with self-signed certs. Pinning is enforced end-to-end (not via pre-flight check) using a small rustls-based electrum client built specifically for pinned connections.
- `deleted_at`: Soft-delete timestamp, `null` for active rows.

## Allowed Roles

- root
- admin

## Examples

### btcmap-cli

```bash
btcmap-cli electrum-server list
btcmap-cli electrum-server list --include-deleted
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_electrum_servers","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
