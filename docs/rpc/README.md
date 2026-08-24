# BTC Map RPC API

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for interacting with BTC Map services.

## Method Categories

- [Auth (STABLE)](auth.md) - Used to create and manage BTC Map accounts
- [Events (STABLE)](event/README.md) - Event management API
- [Areas (WIP)](area/README.md) - Manage community and country data
- [Public Methods](public-methods.md) - Methods for client apps to use
- [Element Methods](element-methods.md) - Methods for working with map elements
- [User Methods](user-methods.md) - Methods for working with user data, including the `set_user_geofence` restriction on event managers
- [Admin Methods](admin-methods.md) - Methods for administrative operations
- [Log Methods](log/README.md) - Methods for log analysis and infrastructure reporting
- [Invoice Methods](invoice-methods.md) - Methods for handling payments
- [Search Methods](search-methods.md) - Methods for searching
- [Electrum servers](electrum/) - Methods for managing electrum servers used by wallet balance lookups
- [Wallets](wallet/) - Methods for managing project wallets and reading their on-chain balances
