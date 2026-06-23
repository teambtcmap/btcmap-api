# whoami

## Description

This method can help you if you forgot your name.

## Output

```json
{
  "name": "satoshi",
  "roles": [
    "user"
  ],
  "registration_date": "2024-06-13T10:33:00Z"
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli auth whoami
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"whoami","id":1}' \
  https://api.btcmap.org/rpc
```