# add_electrum_server

## Description

Adds a new electrum server. Servers with higher `priority` are tried first. URLs must be unique across the table.

## Params

```json
{
  "name": "blockstream",
  "url": "ssl://electrum.blockstream.info:50002",
  "priority": 100,
  "spki_pin": ""
}
```

- `name` (required): Human-readable label.
- `url` (required): Connection URL passed to the electrum client. Supports encrypted `ssl://` and unencrypted `tcp://` URLs.
- `priority` (optional, default `0`): Higher values are tried first.
- `spki_pin` (optional, default empty): For `ssl://` URLs, set to `sha256:<64 hex chars>` to pin a self-signed certificate. The wallet scan will refuse the connection if the server's certificate SPKI doesn't hash to this value. Leave empty for `tcp://` servers and servers with a publicly trusted certificate.

## Result Format

```json
{
  "id": 1,
  "name": "blockstream",
  "url": "ssl://electrum.blockstream.info:50002",
  "priority": 100,
  "spki_pin": "",
  "created_at": "2026-07-24T12:00:00.000Z",
  "updated_at": "2026-07-24T12:00:00.000Z",
  "deleted_at": null
}
```

## Allowed Roles

- root
- admin

## Errors

- The server rejects the call when `url` already exists for an active row.
- Missing `name` or `url` is reported as a JSON-RPC parse error.

## Examples

### btcmap-cli

```bash
btcmap-cli electrum-server add --name foo --url 'ssl://electrum.foo.bar:50002' --priority 10
btcmap-cli electrum-server add \
  --name internal-electrum \
  --url 'ssl://electrum.internal.example:50002' \
  --spki-pin 'sha256:9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"add_electrum_server","params":{"name":"blockstream","url":"ssl://electrum.blockstream.info:50002","priority":100},"id":1}' \
  https://api.btcmap.org/rpc
```
