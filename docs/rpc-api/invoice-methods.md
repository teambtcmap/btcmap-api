
# Invoice RPC Methods

This page documents all RPC methods related to invoices.

## GetInvoice

Retrieves an invoice by its ID.

### Request

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

### Response

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

## GenerateInvoice

Generates a new invoice. Requires admin authentication.

### Request

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

### Response

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

## SyncUnpaidInvoices

Synchronizes the status of unpaid invoices.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "sync_unpaid_invoices",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "invoices_synced": 10,
    "newly_paid": 3
  },
  "id": 1
}
```
