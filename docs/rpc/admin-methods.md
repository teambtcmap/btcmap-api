
# Admin RPC Methods

This document describes the available RPC methods for administrative tasks. All users of these RPC calls are considered Admins and authorization is handled via the `allowed_actions` parameter.

## Table of Contents

- [add_admin](#add_admin) - Add a new admin user
- [add_admin_action](#add_admin_action) - Record an admin action
- [remove_admin_action](#remove_admin_action) - Remove an admin action

## Methods

### add_admin

Adds a new admin.

**Required Admin Action**: `add_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_admin",
  "params": {
    "username": "admin_username",
    "password": "admin_password"
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
    "admin_id": "admin_id"
  },
  "id": 1
}
```

### add_admin_action

Records an admin action.

**Required Admin Action**: `add_admin_action`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_admin_action",
  "params": {
    "action_type": "action_type",
    "entity_id": "entity_id",
    "description": "Action description"
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

### remove_admin_action

Removes an admin action.

**Required Admin Action**: `remove_admin_action`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_admin_action",
  "params": {
    "action_id": "action_id"
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
