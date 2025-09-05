# BTC Map Import RPC

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface for importing merchants from trusted external sources.

## Methods

- [import_merchant](import_merchant.md): Adds new merchants to the map.
- [report_imported_merchant](report_imported_merchant.md): Cancels a pending import or reports that a merchant no longer accepts Bitcoin.

## Authentication

We provide bearer tokens for all trusted sources, you just need to include your token in HTTP request headers.
