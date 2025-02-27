
# Admin RPC Methods

This page documents all RPC methods related to administrative tasks. For details on which admin roles can use these methods, see the [Admin Roles and Permissions](admin-roles.md) documentation.

> **Note:** Some methods require admin authentication but don't have specific role requirements beyond having the method name in the admin's `allowed_actions` list (or having the special "all" permission). These are documented in the [Admin Roles and Permissions](admin-roles.md) document under "Methods Without Role Restrictions".

## AddAdmin

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

## AddAdminAction

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
    "success": true,
    "action_id": "action_id"
  },
  "id": 1
}
```

## RemoveAdminAction

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
