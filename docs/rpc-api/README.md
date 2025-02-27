
# RPC API

The BTCMap API provides a JSON-RPC 2.0 interface for accessing various functionalities.

- [Public Methods](public-methods.md) - Methods available without authentication
- [Admin Methods](admin-methods.md) - Methods for administrative tasks
- [Admin Roles and Permissions](admin-roles.md) - Details about admin roles and their permissions


## Base Endpoint

```
POST /rpc
```

## RPC Request Format

All RPC calls follow the JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": {
    "param1": "value1",
    "param2": "value2"
  },
  "id": 1
}
```

## RPC Response Format

Successful responses:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "key1": "value1",
    "key2": "value2"
  },
  "id": 1
}
```

Error responses:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Error message",
    "data": {}
  },
  "id": 1
}
```

## Authentication

Most RPC methods require authentication. For those methods, you need to include authentication credentials in the request headers or parameters.

The following methods are publicly accessible without authentication:
- GetElement
- PaywallGetAddElementCommentQuote
- PaywallAddElementComment
- PaywallGetBoostElementQuote
- PaywallBoostElement
- GetElementIssues
- GetAreaDashboard
- GetMostActiveUsers

## Available Methods

- [Element Methods](element-methods.md)
- [Area Methods](area-methods.md)
- [User Methods](user-methods.md)
- [Admin Methods](admin-methods.md)
- [Invoice Methods](invoice-methods.md)
- [Search Methods](search-methods.md)
