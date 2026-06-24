# signin

## Description

Use this endpoint to request a new API key. Labels are optional but they will help you manage multiple active keys.

## Params

| Field      | Type            | Required | Description                                                                                          |
| ---------- | --------------- | -------- | ---------------------------------------------------------------------------------------------------- |
| `username` | string          | yes      | The account name to sign in with                                                                     |
| `password` | string          | yes      | The account password                                                                                 |
| `label`    | string          | no       | Optional human-readable label for the issued API key token                                           |
| `roles`    | array of string | no       | Optional list of roles to scope the issued token to. Each name must be a valid `Role`. The methods granted by the requested roles must be a subset of the methods already granted to the signing-in user — omitting the field (or sending `[]`) keeps the legacy behavior of issuing a token that inherits the user's full role set |

```json
{
  "username": "satoshi",
  "password": "qwerty",
  "label": "login with btcmap-cli"
}
```

```json
{
  "username": "satoshi",
  "password": "qwerty",
  "label": "dashboard-app",
  "roles": ["dashboard"]
}
```

## Result

```json
{
  "api_key": "4751a471-b282-4962-8909-fbbf47681b7b",
  "roles": []
}
```

```json
{
  "api_key": "4751a471-b282-4962-8909-fbbf47681b7b",
  "roles": ["dashboard"]
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli auth signin satoshi qwerty
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signin","params":{"username":"satoshi","password":"qwerty"},"id":1}' \
  https://api.btcmap.org/rpc
```

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signin","params":{"username":"satoshi","password":"qwerty","label":"dashboard-app","roles":["dashboard"]},"id":1}' \
  https://api.btcmap.org/rpc
```
