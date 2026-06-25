# Auth RPCs

## Table of Contents

- [change_password](#change_password)
- [get_api_keys](#get_api_keys)
- [revoke_api_key](#revoke_api_key)
- [signin](#signin)
- [signout](#signout)
- [signup](#signup)
- [whoami](#whoami)

## change_password

All users can request a password change. If you received your password from us, we advice you to change it and to store the new password safely in your password manager. User passwords are encrypted at rest using Argon2 KDF. See [change-password.md](auth/change-password.md) for details.

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
    "changed": true
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
btcmap-cli auth change-password --user satoshi --old qwerty --new foobar
```

## get_api_keys

Returns the API keys associated with the authorized user. Secrets are never returned. Each entry exposes the token's effective roles, which may differ from the user's roles when a token was issued with role overrides. See [get_api_keys.md](auth/get_api_keys.md) for details.

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
      "label": "my laptop",
      "roles": ["user"],
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
btcmap-cli auth get-api-keys
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
    "label": "my laptop",
    "revoked_at": "2025-06-19T12:00:00Z"
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
btcmap-cli auth revoke-api-key 1
```

## signin

To enhance security and performance, the BTC Map API avoids requiring your real password for most interactions. Password validation is computationally expensive, and we discourage client applications from caching user credentials. Instead, API calls expect an API key, which you can generate using this method. By default the issued token inherits the signing-in user's full role set; pass an optional `roles` array to mint a token with a narrower scope (e.g. `["dashboard"]` for a read-only analytics token). The requested roles must be a subset of the methods already granted to the user — the API rejects requests that would grant broader access than the user record allows. See [signin.md](auth/signin.md) for details.

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

```json
{
  "jsonrpc": "2.0",
  "method": "signin",
  "params": {
    "username": "satoshi",
    "password": "qwerty",
    "label": "dashboard-app",
    "roles": ["dashboard"]
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
    "roles": []
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
btcmap-cli auth signin satoshi qwerty
```

## signout

Revokes the API key used to call this method. The bearer token is taken from the `Authorization` header, so the caller does not need to know its own key id. After revocation, the token can no longer authenticate any RPC call and is rejected as a `Server error` (the same response given to a non-existent token). See [signout.md](auth/signout.md) for details.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "signout",
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": 1,
    "label": "my laptop",
    "revoked_at": "2025-06-19T12:00:00Z"
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
  --data '{"jsonrpc":"2.0","method":"signout","id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli auth signout
```

## signup

Use this endpoint to create a new BTC Map account. It will return an API key which is required for most RPC API calls. If `username` is omitted, the server generates a random numeric name. The password must be between 12 and 64 characters. See [signup.md](auth/signup.md) for details.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "signup",
  "params": {
    "username": "satoshi",
    "password": "ihsotasatoshi123",
    "label": "sign up with btcmap-cli"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "name": "satoshi",
    "roles": ["user"],
    "api_key": "4751a471-b282-4962-8909-fbbf47681b7b"
  },
  "id": 1
}
```

### Examples

#### curl

```bash
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"signup","params":{"username":"satoshi","password":"ihsotasatoshi123"},"id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli auth signup --user satoshi --password ihsotasatoshi123
```

## whoami

Returns the account name, roles and registration date of the authorized user. See [whoami.md](auth/whoami.md) for details.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "whoami",
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "name": "satoshi",
    "roles": ["user"],
    "registration_date": "2024-06-13T10:33:00Z"
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
  --data '{"jsonrpc":"2.0","method":"whoami","id":1}' \
  https://api.btcmap.org/rpc
```

#### btcmap-cli

```bash
btcmap-cli auth whoami
```