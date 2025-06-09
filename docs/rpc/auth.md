# Auth RPCs

## Table of Contents

- [change_password](#change_password)
- [create_api_key](#create_api_key)

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

## create_api_key

To enhance security and performance, the BTC Map API avoids requiring your real password for most interactions. Password validation is computationally expensive, and we discourage client applications from caching user credentials. Instead, API calls expect an API key, which you can generate using this method.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "create_api_key",
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
    "api_key": "6162641d-c327-4512-811e-1cb08413ab96",
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
  --data '{"jsonrpc":"2.0","method":"create_api_key","params":{"username":"satoshi","password":"qwerty","label":"Created by admin web app on 2025-05-25"},"id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli create_api_key satoshi querty "Created by admin web app on 2025-05-25"
```
