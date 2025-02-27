
# Element RPC Methods

This page documents all RPC methods related to elements.

## GetElement

Retrieves a specific element by its ID.

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
    "name": "Element Name",
    "description": "Element Description",
    "tags": ["tag1", "tag2"],
    "location": {
      "lat": 51.5074,
      "lon": -0.1278
    },
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

## SetElementTag

Adds a tag to an element. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_element_tag",
  "params": {
    "element_id": "element_id",
    "tag": "tag_name"
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

## RemoveElementTag

Removes a tag from an element. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_element_tag",
  "params": {
    "element_id": "element_id",
    "tag": "tag_name"
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

## GetBoostedElements

Retrieves a list of boosted elements.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_boosted_elements",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "elements": [
      {
        "id": "element_id",
        "name": "Element Name",
        "boost_amount": 100,
        "boost_expiration": "2023-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```

## BoostElement

Boosts an element. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "boost_element",
  "params": {
    "element_id": "element_id",
    "amount": 100
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
    "boost_expiration": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

## PaywallGetBoostElementQuote

Gets a quote for boosting an element.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_boost_element_quote",
  "params": {
    "amount": 100
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote": {
      "amount": 100,
      "price_btc": 0.001,
      "price_usd": 50
    }
  },
  "id": 1
}
```

## PaywallBoostElement

Boosts an element through the paywall system.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_boost_element",
  "params": {
    "element_id": "element_id",
    "amount": 100,
    "payment_hash": "payment_hash"
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
    "boost_expiration": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

## AddElementComment

Adds a comment to an element. Requires admin authentication.

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

## PaywallGetAddElementCommentQuote

Gets a quote for adding a comment to an element.

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
      "price_btc": 0.0001,
      "price_usd": 5
    }
  },
  "id": 1
}
```

## PaywallAddElementComment

Adds a comment to an element through the paywall system.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_add_element_comment",
  "params": {
    "element_id": "element_id",
    "content": "Comment content",
    "user_id": "user_id",
    "payment_hash": "payment_hash"
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

## GenerateElementIssues

Generates issues for elements. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_issues",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "issues_generated": 10
  },
  "id": 1
}
```

## SyncElements

Synchronizes elements from an external source. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "sync_elements",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "elements_synced": 100
  },
  "id": 1
}
```

## GenerateElementIcons

Generates icons for elements. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_icons",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "icons_generated": 50
  },
  "id": 1
}
```

## GenerateElementCategories

Generates categories for elements. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_categories",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "categories_generated": 20
  },
  "id": 1
}
```

## GetElementIssues

Retrieves issues for an element.

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
        "created_at": "2023-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```
