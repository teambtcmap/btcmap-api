# get_api_keys

## Description

Returns the list of API keys (access tokens) associated with the authorized user making this request. Secrets are never included in the response, only metadata about each key. Use this to audit your active sessions or find a token id you want to revoke.

Deleted tokens are filtered out of the result.

`last_used_at` records when the token was last presented for authentication. A value of `1970-01-01T00:00:00Z` means the token has never been used.

## Output

```json
[
  {
    "id": 1,
    "label": "my laptop",
    "roles": ["user"],
    "created_at": "2024-06-13T10:33:00Z",
    "updated_at": "2024-06-13T10:33:00Z",
    "last_used_at": "2024-06-14T09:01:00Z"
  },
  {
    "id": 2,
    "label": "ci runner",
    "roles": ["admin", "user"],
    "created_at": "2024-09-01T08:12:00Z",
    "updated_at": "2024-09-01T08:12:00Z",
    "last_used_at": "1970-01-01T00:00:00Z"
  }
]
```

## Examples

### btcmap-cli

```bash
btcmap-cli auth get-api-keys
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_api_keys","id":1}' \
  https://api.btcmap.org/rpc
```