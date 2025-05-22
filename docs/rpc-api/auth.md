# Auth RPCs

## Table of Contents

- [set_password](#set_password)
- [create_auth_token](#create_auth_token)

## set_password

All users are allowed to change their passwords. Your first password might be assigned to you by someone else during manual account creation. You're advised to change you password as soon as possible in that case. User passwords are encrypted at rest using Argon2 KDF.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_password",
  "params": {
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
