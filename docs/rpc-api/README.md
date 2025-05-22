# BTCMap RPC API

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for interacting with BTCMap services.

**WARNING: This API is a work in progress. If you intend to use it without service interruption, please get in touch with us first. Some features may break, especially for use cases we’re unaware of!**

## Method Categories

- [Auth](auth.md)
- [Element Methods](element-methods.md) - Methods for working with map elements
- [Area Methods](area-methods.md) - Methods for working with geographic areas
- [User Methods](user-methods.md) - Methods for working with user data
- [Admin Methods](admin-methods.md) - Methods for administrative operations
- [Invoice Methods](invoice-methods.md) - Methods for handling payments
- [Search Methods](search-methods.md) - Methods for searching

## Authentication

Most RPC methods require appropriate admin authentication and authorization. Authentication is handled via an API key that must be included in the request headers.

Admins must have the appropriate `allowed_actions` set for the specific methods they want to call. Public methods can be called without authentication.

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
