# Invoices REST API (v4)

This document describes the endpoints for interacting with invoices in REST API v4.

## Available Endpoints

- [Get by ID](#get-by-id)

### Get by ID

```
curl https://api.btcmap.org/v4/invoices/{id}
```

Retrieves a specific invoice by its ID.

#### Path Parameters

| Parameter | Type | Example | Default | Required |
|-----------|------|---------|---------|-------------|
| `id` | String | `dd79bb72-6535-4ada-a683-88b6e8550f14` | - | **Yes** |

#### Examples

##### Check Invoice Status

```
curl https://api.btcmap.org/v4/invoices/dd79bb72-6535-4ada-a683-88b6e8550f14 | jq
```

```json
{
  "id": "dd79bb72-6535-4ada-a683-88b6e8550f14",
  "status": "unpaid"
}
```