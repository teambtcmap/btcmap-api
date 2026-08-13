# BTC Map RPC API

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for interacting with BTCMap services.

## Method Categories

- [Auth (STABLE)](auth.md) - Used to create and manage BTC Map accounts
- [Events (STABLE)](event/README.md) - Event management API
- [Area Methods (WIP)](area/README.md) - Methods for working with community and country data
- [Public Methods](public-methods.md) - Methods for client apps to use
- [Element Methods](element-methods.md) - Methods for working with map elements
- [User Methods](user-methods.md) - Methods for working with user data, including the `set_user_geofence` restriction on event managers
- [Admin Methods](admin-methods.md) - Methods for administrative operations
- [Log Methods](log/README.md) - Methods for log analysis and infrastructure reporting
- [Invoice Methods](invoice-methods.md) - Methods for handling payments
- [Search Methods](search-methods.md) - Methods for searching
- [Electrum servers](electrum/) - Methods for managing electrum servers used by wallet balance lookups
- [Wallets](wallet/) - Methods for managing project wallets and reading their on-chain balances

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
