# BTC Map Import RPC

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for importing locations from trusted external sources.

## Methods

- [submit_place](submit_place.md): Adds new places to the map.
- [get_submitted_place](get_submitted_place.md): Fetch previous submission details.
- [revoke_submitted_place](revoke_submitted_place.md): Cancels a pending import or reports that a place no longer accepts Bitcoin.

## Authentication

We provide bearer tokens for all trusted sources, you just need to include your token in HTTP request headers.
