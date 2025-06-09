# Admin Allowed Actions

This document outlines the different administrative allowed actions available in the system.

## Action Types

Administrators can be granted specific allowed actions that permit them to perform certain operations.  All administrators are considered to have admin privileges; their access is controlled by the `allowed_actions` list associated with their account.


## Allowed Actions Matrix

The following table shows which allowed actions are necessary to use each admin method:

| Method | Required Allowed Actions |
|--------|--------------------------|
| `add_admin` | `add_admin`             |
| `add_admin_action` | `add_admin_action`       |
| `remove_admin_action` | `remove_admin_action`     |
| `set_element_tag` | `set_element_tag`         |
| `remove_element_tag` | `remove_element_tag`       |
| `delete_element` | `delete_element`         |
| `boost_element` | `boost_element`           |
| `ban_user` | `ban_user`               |
| `unban_user` | `unban_user`             |
| `set_user_tag` | `set_user_tag`           |
| `remove_user_tag` | `remove_user_tag`         |
| `get_admin_logs` | `get_admin_logs`         |
| `get_admin_stats` | `get_admin_stats`         |

## Methods Requiring Only Admin Authentication

The following admin methods require admin authentication but don't have specific allowed action requirements beyond having the method name in the admin's `allowed_actions` list (or having the special "all" permission):

| Method | Description |
|--------|-------------|
| `generate_reports` | Generates statistical reports about elements |
| `generate_element_issues` | Analyzes elements and generates issues for those with problems |
| `generate_areas_elements_mapping` | Creates mappings between areas and elements based on geographic boundaries |
| `sync_unpaid_invoices` | Synchronizes unpaid invoice statuses with payment processors |

## How Permissions Are Enforced

When an admin makes an RPC request, the system checks:

1. If the admin is authenticated
2. If the admin has the required allowed action to perform the requested action (or the "all" permission)
3. If the admin has any restrictions or limitations on their account

If any of these checks fail, the request will be rejected with an appropriate error message.

## Public Methods

The following RPC methods are publicly accessible and do not require any admin authentication:

| Method | Description |
|--------|-------------|
| `get_element` | Retrieves element details |
| `paywall_get_add_element_comment_quote` | Gets a quote for adding an element comment |
| `paywall_add_element_comment` | Adds a comment to an element via paywall |
| `paywall_get_boost_element_quote` | Gets a quote for boosting an element |
| `paywall_boost_element` | Boosts an element via paywall |
| `get_element_issues` | Retrieves issues related to an element |
| `get_area_dashboard` | Gets dashboard information for an area |
| `get_most_active_users` | Retrieves a list of most active users |


## Methods Requiring Only Admin Authentication

The following methods require admin authentication but do not have specific allowed action requirements beyond having the method name in the admin's `allowed_actions` list (or having the special "all" permission):

| Method | Description |
|--------|-------------|
| `get_user_activity` | Gets activity data for a user |
| `get_invoice` | Retrieves invoice details |
| `search` | Performs a search query |
| `set_element_tag` | Sets a tag on an element |
| `remove_element_tag` | Removes a tag from an element |
| `boost_element` | Boosts an element (admin version) |
| `sync_elements` | Syncs elements data |
| `generate_element_icons` | Generates icons for elements |
| `generate_element_categories` | Generates categories for elements |
| `generate_element_issues` | Generates issues for elements |
| `add_area` | Adds a new area |
| `get_area` | Gets details of an area |
| `set_area_tag` | Sets a tag on an area |
| `remove_area_tag` | Removes a tag from an area |
| `set_area_icon` | Sets an icon for an area |
| `remove_area` | Removes an area |
| `generate_areas_elements_mapping` | Generates mappings between areas and elements |
| `generate_reports` | Generates reports |
| `set_user_tag` | Sets a tag on a user |
| `remove_user_tag` | Removes a tag from a user |
| `generate_invoice` | Generates a new invoice |
| `sync_unpaid_invoices` | Syncs status of unpaid invoices |

## Adding Allowed Actions to Admin Accounts

Allowed actions are stored in the `allowed_actions` field of the Admin database record.  To add allowed actions to an admin, use the `add_admin_action` or `remove_admin_action` RPC methods (These methods themselves have their own `allowed_actions` requirements).


```json
{
  "jsonrpc": "2.0",
  "method": "add_admin_action",
  "params": {
    "admin_id": 123,
    "action": "boost_element" 
  },
  "id": 1
}
```

```json
{
  "jsonrpc": "2.0",
  "method": "remove_admin_action",
  "params": {
    "admin_id": 123,
    "action": "boost_element"
  },
  "id": 1
}