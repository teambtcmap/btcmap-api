
# Public RPC Methods

These RPC methods are publicly accessible and do not require any admin authentication.

## GetElement

Retrieves information about a specific element.

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

## PaywallGetAddElementCommentQuote

Gets a quote for adding a comment to an element through the paywall.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_add_element_comment_quote",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote": {
      "amount": "amount_in_sats",
      "expiry": "expiry_time"
    }
  },
  "id": 1
}
```

## PaywallAddElementComment

Adds a comment to an element through the paywall.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_add_element_comment",
  "params": {
    "element_id": "element_id",
    "comment": "Comment text",
    "username": "username",
    "payment_hash": "lightning_payment_hash"
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

## PaywallGetBoostElementQuote

Gets a quote for boosting an element through the paywall.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_boost_element_quote",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote": {
      "amount": "amount_in_sats",
      "expiry": "expiry_time"
    }
  },
  "id": 1
}
```

## PaywallBoostElement

Boosts an element through the paywall.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_boost_element",
  "params": {
    "element_id": "element_id",
    "username": "username",
    "payment_hash": "lightning_payment_hash"
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
    "boost_id": "boost_id"
  },
  "id": 1
}
```

## GetElementIssues

Retrieves issues related to a specific element.

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

## GetAreaDashboard

Retrieves dashboard information for a specific area.

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

## GetMostActiveUsers

Retrieves a list of the most active users.

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
