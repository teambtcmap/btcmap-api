# signup

## Description

Use this endpoint to create a new BTC Map account. It will return an API key which is required for most RPC API calls.

## Params

| Field       | Type   | Required | Description                                                  |
| ----------- | ------ | -------- | ------------------------------------------------------------ |
| `username`  | string | yes      | The account name to register                                 |
| `password`  | string | yes      | The account password                                         |
| `label`     | string | no       | Optional human-readable label for the issued API key token   |

```json
{
  "username": "satoshi",
  "password": "qwerty",
  "label": "sign up with btcmap-cli"
}
```

## Result

```json
{
  "name": "satoshi",
  "roles": [
    "user"
  ],
  "api_key": "4751a471-b282-4962-8909-fbbf47681b7b"
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli signup --user satoshi --password qwerty
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signup","params":{"username":"satoshi","password":"qwerty"},"id":1}' \
  https://api.btcmap.org/rpc
```
