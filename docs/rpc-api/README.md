
# BTCMap RPC API

The RPC API provides a JSON-RPC interface for interacting with BTCMap services.

## Method Categories

- [Element Methods](element-methods.md) - Methods for working with map elements
- [Area Methods](area-methods.md) - Methods for working with geographic areas
- [User Methods](user-methods.md) - Methods for working with user data
- [Admin Methods](admin-methods.md) - Methods for administrative operations
- [Invoice Methods](invoice-methods.md) - Methods for handling payments
- [Search Methods](search-methods.md) - Methods for searching

## Authentication

Most RPC methods require authentication with an admin password. This can be provided either:

1. In the request parameters as `password`
2. As a `Bearer` token in the `Authorization` header

## Request Format

All RPC requests should be POST requests to the `/rpc` endpoint with a JSON body following the JSON-RPC 2.0 specification:

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

## Response Format

Responses follow the JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "key": "value"
  },
  "id": 1
}
```

Or in case of an error:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Server error",
    "data": "Error details"
  },
  "id": 1
}
```
