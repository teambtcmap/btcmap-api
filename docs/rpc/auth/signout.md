# signout

## Description

Revokes the API key used to call this method. The bearer token is taken from the `Authorization` header, so the caller does not need to know its own key id. Use this when the user explicitly signs out of a client application.

Once revoked, the token can no longer authenticate any RPC call: subsequent requests with the same bearer are rejected with a JSON-RPC `Server error` (code `-32000`), the same response given to a non-existent token.

## Args

This method takes no parameters.

```json
{}
```

## Response

```json
{
  "id": 1,
  "label": "my laptop",
  "revoked_at": "2025-06-19T12:00:00Z"
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli auth signout
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signout","id":1}' \
  https://api.btcmap.org/rpc
```
