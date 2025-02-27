# User Methods

This document describes the available RPC methods for interacting with users.

## Available Methods

- [get_users](#get_users) - Retrieve users based on query parameters
- [get_user_by_id](#get_user_by_id) - Retrieve a specific user by ID
- [get_most_active_users](#get_most_active_users) - Get the most active users
- [get_user_activity](#get_user_activity) - Get activity data for a specific user
- [set_user_tag](#set_user_tag) - Set a tag for a user
- [remove_user_tag](#remove_user_tag) - Remove a tag from a user

## Methods

### get_users

Retrieves users based on query parameters.

**Required Admin Action**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_users",
  "params": {
    "updated_since": "2023-01-01T00:00:00Z",
    "limit": 10
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "users": [
      {
        "id": 123,
        "display_name": "username",
        "created_at": "2020-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```

### get_user_by_id

Retrieves a specific user by their ID.

**Required Admin Action**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_user_by_id",
  "params": {
    "id": 123
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "user": {
      "id": 123,
      "display_name": "username",
      "created_at": "2020-01-01T00:00:00Z"
    }
  },
  "id": 1
}
```

### get_most_active_users

Retrieves the most active users on the platform.

**Required Admin Action**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_most_active_users",
  "params": {
    "limit": 10
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "users": [
      {
        "id": 123,
        "name": "User Name",
        "activity_score": 95,
        "comments": 50,
        "elements_added": 20
      }
    ]
  },
  "id": 1
}
```

### ban_user

Bans a user from the platform.

**Required Admin Action**: `user:ban`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "ban_user",
  "params": {
    "password": "your_admin_password",
    "user_id": 123,
    "reason": "Violation of terms of service",
    "duration_days": 30
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "ban_expires": "2023-07-15T00:00:00Z"
  },
  "id": 1
}
```

### unban_user

Removes a ban from a user.

**Required Admin Action**: `user:unban`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "unban_user",
  "params": {
    "password": "your_admin_password",
    "user_id": 123
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}