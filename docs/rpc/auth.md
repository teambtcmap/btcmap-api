# Auth RPCs

## Table of Contents

- [change_password](#change_password)
- [get_api_keys](#get_api_keys)
- [revoke_api_key](#revoke_api_key)
- [signin](#signin)

## change_password

All users can request a password change. If you received your password from us, we advice you to change it and to store the new password safely in your password manager. User passwords are encrypted at rest using Argon2 KDF.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "change_password",
  "params": {
    "username": "satoshi",
    "old_password": "qwerty",
    "new_password": "foobar"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "time_ms": 123
  },
  "id": 1
}
```

### Examples

#### curl

```bash
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"change_password","params":{"username":"satoshi","old_password":"qwerty","new_password":"foobar"},"id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli change-password satoshi querty foobar
```

## get_api_keys

Returns the API keys associated with the authorized user. Secrets are never returned. See [get_api_keys.md](auth/get_api_keys.md) for details.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_api_keys",
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "id": 1,
      "name": "my laptop",
      "roles": ["user"],
      "import_origins": [],
      "created_at": "2024-06-13T10:33:00Z",
      "updated_at": "2024-06-13T10:33:00Z"
    }
  ],
  "id": 1
}
```

### Examples

#### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_api_keys","id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli get-api-keys
```

## revoke_api_key

Revokes an API key by its id. The key must belong to the authorized user. See [revoke_api_key.md](auth/revoke_api_key.md) for details.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "revoke_api_key",
  "params": {
    "id": 1
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": 1,
    "revoked": true,
    "deleted_at": "2025-06-19T12:00:00Z"
  },
  "id": 1
}
```

### Examples

#### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $API_KEY" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"revoke_api_key","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli revoke-api-key 1
```

## signin

To enhance security and performance, the BTC Map API avoids requiring your real password for most interactions. Password validation is computationally expensive, and we discourage client applications from caching user credentials. Instead, API calls expect an API key, which you can generate using this method.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "signin",
  "params": {
    "username": "satoshi",
    "password": "qwerty",
    "label": "Created by admin web app on 2025-05-25"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "token": "6162641d-c327-4512-811e-1cb08413ab96"
  },
  "id": 1
}
```

### Examples

#### curl

```bash
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signin","params":{"username":"satoshi","password":"qwerty","label":"Created by admin web app on 2025-05-25"},"id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli create_api_key satoshi querty "Created by admin web app on 2025-05-25"
```
