# whoami

## Description

This method can help you if you forgot your name.

## Output

```json
{
  "name": "Satoshi",
  "roles": [
    "root"
  ],
  "created_at": "2024-06-13T10:33:73Z"
}
```

## Allowed Roles

- User
- Admin
- Root

## Examples

### btcmap-cli

```bash
btcmap-cli rpc whoami
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"whoami","id":1}' \
  https://api.btcmap.org/rpc
```