# create_api_key

## Description

You need to get an API key in order to use most RPC endpoints. Creating a new key does not invalidate the existing keys.

## Params

```json
{
  "username": "Satoshi",
  "password": "SuperSecurePassword",
  "label": "test"
}
```

Note: `label` is an optional field.

## Result

```json
{
  "token": "4751a471-b282-4962-8909-fbbf47681b7b",
  "time_ms": 3
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli create-api-key Satoshi SuperSecurePassword
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_api_key","params":{"username":"Satoshi","password":"SuperSecurePassword"},"id":1}' \
  https://api.btcmap.org/rpc
```