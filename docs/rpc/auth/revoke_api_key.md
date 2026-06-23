# revoke_api_key

## Description

Revokes an API key by its id. Use this to remove keys you no longer use or suspect have been exposed.

The operation is idempotent: revoking an already-revoked key returns a response with `revoked_at` timestamp.

## Args

```json
{
  "id": 1
}
```

## Response

```json
{
  "id": 1,
  "label": "satoshi's laptop",
  "revoked_at": "2025-06-19T12:00:00Z"
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli revoke-api-key 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"revoke_api_key","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```