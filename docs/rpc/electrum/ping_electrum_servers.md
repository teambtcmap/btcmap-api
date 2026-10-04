# ping_electrum_servers

## Description

Sends the standard Electrum JSON-RPC `server.ping` request to every configured
electrum server and reports the outcome per server. Servers are probed in
parallel, so the call returns as soon as the slowest server has answered (or
its 5-second per-server timeout has elapsed). Use this to quickly see which
backends are reachable and how fast they respond.

The endpoint never fails because of a single unhealthy server: each result
entry carries its own `ok` flag and an optional `error` string. The outer
JSON-RPC call only errors if the database lookup itself fails.

## Params

```json
{
  "include_deleted": false
}
```

- `include_deleted` (optional, default `false`): when `true`, soft-deleted
  rows are included in the result so you can probe a server that has been
  removed from the active set but is still around for diagnostic purposes.

## Result Format

```json
[
  {
    "id": 1,
    "name": "Internal 1",
    "url": "ssl://electrum-1.btcmap.org:50002",
    "priority": 100,
    "ok": true,
    "latency_ms": 84
  },
  {
    "id": 2,
    "name": "Blockstream",
    "url": "ssl://electrum.blockstream.info:50002",
    "priority": 0,
    "ok": false,
    "error": "electrum client connect failed for ssl://electrum.blockstream.info:50002: timeout"
  }
]
```

## Fields

- `id`: Server id, same value as returned by `get_electrum_servers`.
- `name`: Human-readable label.
- `url`: Connection URL that was probed.
- `priority`: Higher values are tried first. Mirrors `get_electrum_servers`.
- `ok`: `true` when the server replied with the expected `null` result to
  `server.ping`; `false` when the connect timed out, the SPKI pin did not
  match, the server closed the connection, or anything else went wrong.
- `latency_ms`: Round-trip time in milliseconds, only present when `ok` is
  `true`. Measured from connect start to a successful ping response.
- `error`: Human-readable error description, only present when `ok` is
  `false`. Comes straight from the underlying client so TLS / SPKI / network
  issues are surfaced verbatim.

## Allowed Roles

- root
- admin

## Examples

### btcmap-cli

```bash
btcmap-cli electrum-server ping
btcmap-cli electrum-server ping --include-deleted
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"ping_electrum_servers","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
