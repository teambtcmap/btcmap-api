
# Invoice RPC Methods

This document describes the available RPC methods for interacting with invoices.

## Table of Contents

- [get_invoice](#get_invoice) - Retrieve a specific invoice by ID
- [generate_invoice](#generate_invoice) - Generate a new invoice
- [sync_unpaid_invoices](#sync_unpaid_invoices) - Synchronize status of unpaid invoices

## Methods

### get_invoice

Retrieves an invoice by its ID.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_invoice",
  "params": {
    "invoice_id": "invoice_id"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": "invoice_id",
    "payment_hash": "payment_hash",
    "payment_request": "lightning_invoice",
    "amount_sats": 1000,
    "description": "Invoice description",
    "status": "paid",
    "created_at": "2023-01-01T00:00:00Z",
    "paid_at": "2023-01-01T00:01:00Z"
  },
  "id": 1
}
```

### generate_invoice

Generates a new invoice.

**Required Admin Action**: `generate_invoice`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_invoice",
  "params": {
    "amount_sats": 1000,
    "description": "Invoice description",
    "entity_type": "entity_type",
    "entity_id": "entity_id"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": "invoice_id",
    "payment_hash": "payment_hash",
    "payment_request": "lightning_invoice",
    "amount_sats": 1000,
    "description": "Invoice description",
    "status": "pending",
    "created_at": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

### sync_unpaid_invoices

Synchronizes the status of unpaid invoices.

**Required Admin Action**: `sync_unpaid_invoices`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "sync_unpaid_invoices",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "invoices_synced": 5,
    "invoices_paid": 2
  },
  "id": 1
}
```
