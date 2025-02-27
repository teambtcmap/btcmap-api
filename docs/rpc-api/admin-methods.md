
# Admin RPC Methods

This page documents all RPC methods related to administrative tasks. For details on which admin roles can use these methods, see the [Admin Roles and Permissions](admin-roles.md) documentation.

## Available Methods

- [add_admin](#add_admin) - Add a new admin user
- [add_admin_action](#add_admin_action) - Record an admin action
- [remove_admin_action](#remove_admin_action) - Remove an admin action

## add_admin

Adds a new admin. Requires `super_admin` role.

### Request

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

### Response

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

## add_admin_action

Records an admin action. Requires `super_admin`, `moderator`, `content_manager`, or `reviewer` role.

### Request

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

## remove_admin_action

Removes an admin action. Requires `super_admin` role.

### Request

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
