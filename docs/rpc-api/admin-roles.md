
# Admin Roles and Permissions

This document outlines the different admin roles and their associated permissions for using admin RPC methods.

## Admin Roles

The BTCMap API supports the following admin roles:

| Role | Description |
|------|-------------|
| `super_admin` | Has full access to all admin methods |
| `moderator` | Can moderate content and manage basic elements |
| `content_manager` | Can manage content but not users or system settings |
| `reviewer` | Can review and approve content submissions |
| `read_only` | Can view admin data but cannot make changes |

## Role Permissions Matrix

The following table shows which admin roles are allowed to use each admin method:

| Method | super_admin | moderator | content_manager | reviewer | read_only |
|--------|-------------|-----------|-----------------|----------|-----------|
| `add_admin` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `add_admin_action` | ✅ | ✅ | ✅ | ✅ | ❌ |
| `remove_admin_action` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `set_element_tag` | ✅ | ✅ | ✅ | ❌ | ❌ |
| `remove_element_tag` | ✅ | ✅ | ✅ | ❌ | ❌ |
| `delete_element` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `boost_element` | ✅ | ✅ | ✅ | ❌ | ❌ |
| `ban_user` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `unban_user` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `set_user_tag` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `remove_user_tag` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `get_admin_logs` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `get_admin_stats` | ✅ | ✅ | ✅ | ✅ | ✅ |

## How Permissions Are Enforced

When an admin makes an RPC request, the system checks:

1. If the admin is authenticated
2. If the admin has the required role to perform the requested action
3. If the admin has any restrictions or limitations on their account

If any of these checks fail, the request will be rejected with an appropriate error message.

## Adding Role Requirements to Admin Accounts

Roles are stored in the `allowed_actions` field of the Admin database record. To assign roles to an admin, use the `set_admin_role` RPC method (restricted to super_admin):

```json
{
  "jsonrpc": "2.0",
  "method": "set_admin_role",
  "params": {
    "admin_id": 123,
    "role": "moderator" 
  },
  "id": 1
}
```
