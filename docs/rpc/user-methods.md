# User Methods

This document describes the available RPC methods for interacting with users.

## Table of Contents

- [get_user_activity](#get_user_activity) - Get activity data for a specific user
- [set_user_tag](#set_user_tag) - Set a tag for a user
- [set_user_geofence](#set_user_geofence) - Restrict where an event manager may operate
- [remove_user_tag](#remove_user_tag) - Remove a tag from a user
- [get_most_active_users](#get_most_active_users) - Get the most active users
- [get_users](#get_users) - Retrieve users based on query parameters
- [get_user_by_id](#get_user_by_id) - Retrieve a specific user by ID
- [ban_user](#ban_user) - Bans a user from the platform.
- [unban_user](#unban_user) - Removes a ban from a user.


## Methods

### get_user_activity

Get activity data for a specific user.

**Required Admin Action**: `user_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_user_activity",
  "params": {
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
    "activity": [
      {
        "type": "element_comment",
        "element_id": 456,
        "comment_id": 789,
        "timestamp": "2023-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```

### set_user_tag

Set a tag for a user.

**Required Admin Action**: `user_admin`

#### Request

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

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### set_user_geofence

Sets the geofence — a whitelist of area ids — that constrains where the target
user is allowed to manage events. When the target user has a non-empty
geofence, the restriction applies equally to root, admin, and event manager
roles.

When the geofence is non-empty, the target user is allowed to create, update
or delete an event only if:

- the event is linked to an `area_id` that is in the geofence, **or**
- the event's `(lat, lon)` falls inside the polygon (or multi-polygon / line
  string) of at least one area in the geofence.

Pass an empty array to clear the geofence and lift the restriction. The
response echoes the new state.

**Required Admin Action**: `admin` (only admins and roots may call this).

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_user_geofence",
  "params": {
    "user_name": "alice",
    "geofence": [1, 5, 14]
  },
  "id": 1
}
```

| Field       | Type            | Description                                                                                       |
| ----------- | --------------- | ------------------------------------------------------------------------------------------------- |
| `user_name` | string          | Required. Username of the user whose geofence is being set.                                        |
| `geofence`  | array of ints   | Required. Area ids the user is allowed to operate in. Pass `[]` to clear the restriction entirely. |

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": 19,
    "name": "alice",
    "geofence": [1, 5, 14]
  },
  "id": 1
}
```

#### Error cases

- If `user_name` does not match an existing user, the call returns a server
  error.
- When the target user tries to create, update, or delete an event outside their
  geofence, the event-mutating call rejects with a message like
  `"Area 999 is outside your geofence (allowed: [1, 5, 14])"` or
  `"Location (51.5, -0.1) is outside your geofence (allowed areas: [1, 5, 14])"`.

#### Examples

##### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ADMIN_ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"set_user_geofence","params":{"user_name":"alice","geofence":[1,5,14]},"id":1}' \
  https://api.btcmap.org/rpc
```

### remove_user_tag

Remove a tag from a user.

**Required Admin Action**: `user_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_user_tag",
  "params": {
    "user_id": 123,
    "tag": "contributor"
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
```

### get_most_active_users

Get the most active users.

**Required Admin Action**: None (publicly accessible)

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
        "display_name": "username",
        "activity_count": 42
      }
    ]
  },
  "id": 1
}
```

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