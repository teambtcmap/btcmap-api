
# Public RPC Methods

These methods are intended to be used by user-facing apps, such as the Web App, the iOS App and the Android App.

## Table of Contents

- [AddElementComment](#addelementcomment)
- [BoostElement](#boostelement)
- [GetArea](#getarea)
- [GetAreaDashboard](#getareadashboard)
- [GetElement](#getelement)
- [GetElementIssues](#getelementissues)
- [GetInvoice](#getinvoice)
- [GetMostActiveUsers](#getmostactiveusers)
- [PaywallGetAddElementCommentQuote](#paywallgetaddelementcommentquote)
- [PaywallAddElementComment](#paywalladdelementcomment)
- [PaywallGetBoostElementQuote](#paywallgetboostelementquote)
- [PaywallBoostElement](#paywallboostelement)
- [Search](#search)

## Methods

### AddElementComment

Adds a comment to an element.

Should only really be used by admin apps. User-facing apps should use [PaywallGetAddElementCommentQuote](#paywallgetaddelementcommentquote) and [PaywallBoostElement](#paywallboostelement).

**Authentication**: Requires admin authentication

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_element_comment",
  "params": {
    "element_id": "element_id",
    "content": "Comment content",
    "user_id": "user_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "comment_id": "comment_id"
  },
  "id": 1
}
```

### BoostElement

Boosts an element.

Should only really be used by admin apps. User-facing apps should use [PaywallGetBoostElementQuote](#paywallgetboostelementquote) and [PaywallAddElementComment](#paywalladdelementcomment).

**Authentication**: Requires admin authentication with 'boost_element' action

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "boost_element",
  "params": {
    "element_id": "element_id",
    "days": 30
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### GetArea

Retrieves information about an area.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area",
  "params": {
    "id": "area_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": "area_id",
    "name": "Area Name",
    "description": "Area description",
    "coordinates": {
      "lat": 12.345,
      "lng": 67.890
    }
  },
  "id": 1
}
```

### GetAreaDashboard

Gets dashboard information for a specific area.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area_dashboard",
  "params": {
    "area_id": "area_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "area_id": "area_id",
    "name": "Area Name",
    "element_count": 123,
    "recent_elements": [],
    "top_contributors": [],
    "recent_activity": []
  },
  "id": 1
}
```

### GetElement

Retrieves information about a specific element.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_element",
  "params": {
    "id": "element_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": "element_id",
    "type": "element_type",
    "name": "Element Name",
    "description": "Element description",
    "tags": ["tag1", "tag2"],
    "coordinates": {
      "lat": 12.345,
      "lng": 67.890
    },
    "created_at": "timestamp",
    "updated_at": "timestamp"
  },
  "id": 1
}
```

### GetElementIssues

Retrieves issues related to a specific element.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_element_issues",
  "params": {
    "element_id": "element_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "issues": [
      {
        "id": "issue_id",
        "element_id": "element_id",
        "type": "issue_type",
        "description": "Issue description",
        "status": "open",
        "created_at": "timestamp",
        "updated_at": "timestamp"
      }
    ]
  },
  "id": 1
}
```

### GetInvoice

Gets information about an invoice by its ID.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_invoice",
  "params": {
    "invoice_id": "inv_12345"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "invoice": {
      "id": "inv_12345",
      "status": "paid",
      "amount_btc": 0.001,
      "amount_usd": 50,
      "created_at": "2023-06-15T14:30:00Z",
      "paid_at": "2023-06-15T14:35:00Z"
    }
  },
  "id": 1
}
```

### GetMostActiveUsers

Retrieves a list of the most active users.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_most_active_users",
  "params": {
    "limit": 10
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "users": [
      {
        "username": "username",
        "activity_count": 123,
        "last_active": "timestamp"
      }
    ]
  },
  "id": 1
}
```

### PaywallAddElementComment

Adds a comment to an element through the paywall system.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_add_element_comment",
  "params": {
    "element_id": "123456",
    "comment": "This is a great place!"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "payment_request": "lnbc..."
  },
  "id": 1
}
```

### PaywallBoostElement

Boosts an element through the paywall system.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_boost_element",
  "params": {
    "element_id": "123456",
    "days": 30
  },
  "id": 1
}
```

Note: The `days` parameter must be one of: 30, 90, or 365.

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "payment_request": "lnbc..."
  },
  "id": 1
}
```

### PaywallGetAddElementCommentQuote

Gets a quote for adding a comment to an element.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0", 
  "method": "paywall_get_add_element_comment_quote",
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote_sat": 100
  },
  "id": 1
}
```

### PaywallGetBoostElementQuote

Gets quotes for boosting an element for different durations.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_boost_element_quote",
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote_30d_sat": 1000,
    "quote_90d_sat": 2500,
    "quote_365d_sat": 8000
  },
  "id": 1
}
```

### Search

Searches for elements, users, and other entities based on search terms.

**Authentication**: No authentication required

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "search",
  "params": {
    "query": "coffee shop",
    "limit": 10,
    "types": ["element", "user"]
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "items": [
      {
        "type": "element",
        "id": "123",
        "name": "Bitcoin Coffee Shop",
        "description": "Coffee shop that accepts Bitcoin",
        "score": 0.95
      },
      {
        "type": "user",
        "id": "456",
        "name": "CoffeeShopOwner",
        "score": 0.75
      }
    ]
  },
  "id": 1
}
```