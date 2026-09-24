# BTC Map RPC API

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for interacting with BTC Map services.

## Method Categories

- [Auth (STABLE)](auth.md) - Used to create and manage BTC Map accounts; per-method docs in [auth/](auth/) (signin, signup, signout, password change, API keys, whoami)
- [Events (STABLE)](event/README.md) - Event management API; per-method docs in [event/](event/) (create, read, update, soft-delete events with geofence enforcement)
- [Areas (WIP)](area/README.md) - Manage community and country data; per-method docs in [area/](area/) (CRUD, tags, images, trending and dashboard reports, element mapping)
- [Public Methods](public-methods.md) - Methods for client apps to use
- [Element Methods](element-methods.md) - Methods for working with map elements
- [User Methods](user-methods.md) - Methods for working with user data, including the `set_user_geofence` restriction on event managers
- [Admin Methods](admin-methods.md) - Methods for administrative operations
- [Analytics](analytics/README.md) - Dashboards and analytical reports: place statistics, daily infrastructure and request-log reports, top API consumers
- [Import](import/README.md) - Add and manage places submitted by trusted external sources; per-method docs in [import/](import/) (`submit_place`, `revoke_submitted_place`, `get_submitted_place`, `get_place_import_origins`)
- [Invoice Methods](invoice-methods.md) - Methods for handling payments; per-method docs in [invoice/](invoice/) (create and read invoices)
- [Search Methods](search-methods.md) - Methods for searching
- [Electrum servers](electrum/) - Methods for managing electrum servers used by wallet balance lookups; per-method docs in [electrum/](electrum/) (list, add, update, remove)
- [Wallets](wallet/) - Methods for managing project wallets and reading their on-chain balances; per-method docs in [wallet/](wallet/) (list, add, update, soft-delete)
