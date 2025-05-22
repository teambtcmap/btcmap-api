# Auth RPCs

## Table of Contents

- [change_password](#change_password)
- [create_auth_token](#create_auth_token)

## change_password

All users can request a password change. If you received your password from us, we advice you to change it and to store the new password safely in your password manager. User passwords are encrypted at rest using Argon2 KDF.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_password",
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

### Example: curl

```bash
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"change_password","params":{"username":"satoshi","old_password":"qwerty","new_password":"foobar"},"id":1}' \
  https://api.btcmap.org/rpc
```

## create_auth_token

Set a tag for a user.

**Required Admin Action**: `user_admin`

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_user_tag",
  "params": {
    "user_id": 123,
    "tag": "contributor"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```
