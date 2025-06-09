
# Element Methods

This document describes the available RPC methods for interacting with elements.

## Table of Contents

- [get_element](#get_element) - Retrieve a specific element by ID
- [set_element_tag](#set_element_tag) - Set a tag on an element
- [remove_element_tag](#remove_element_tag) - Remove a tag from an element
- [get_boosted_elements](#get_boosted_elements) - Get elements that have been boosted
- [boost_element](#boost_element) - Boost an element
- [paywall_get_boost_element_quote](#paywall_get_boost_element_quote) - Get a quote for boosting an element
- [paywall_boost_element](#paywall_boost_element) - Boost an element with payment
- [add_element_comment](#add_element_comment) - Add a comment to an element
- [paywall_get_add_element_comment_quote](#paywall_get_add_element_comment_quote) - Get a quote for adding a comment
- [paywall_add_element_comment](#paywall_add_element_comment) - Add a comment with payment
- [generate_element_issues](#generate_element_issues) - Generate issues for elements
- [sync_elements](#sync_elements) - Synchronize elements with external source
- [generate_element_icons](#generate_element_icons) - Generate icons for elements
- [generate_element_categories](#generate_element_categories) - Generate categories for elements
- [get_element_issues](#get_element_issues) - Get issues associated with elements

## Methods

### get_element

Retrieves a specific element by its ID.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_element",
  "params": {
    "id": 123456
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "element": {
      "id": 123456,
      "osm_type": "node",
      "osm_id": 123456,
      "tags": {
        "name": "Bitcoin Coffee",
        "amenity": "cafe",
        "currency:XBT": "yes"
      }
    }
  },
  "id": 1
}
```

### set_element_tag

Adds a tag to an element.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_element_tag",
  "params": {
    "element_id": "123456",
    "tag": "featured"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### remove_element_tag

Removes a tag from an element.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_element_tag",
  "params": {
    "element_id": "123456",
    "tag": "featured"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### get_boosted_elements

Get elements that have been boosted.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_boosted_elements",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "elements": [
      {
        "id": 123456,
        "osm_type": "node",
        "osm_id": 123456,
        "tags": {
          "name": "Bitcoin Coffee",
          "amenity": "cafe",
          "currency:XBT": "yes"
        }
      }
    ]
  },
  "id": 1
}
```

### boost_element

Boosts an element.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "boost_element",
  "params": {
    "element_id": "123456",
    "amount": 1000
  },
  "id": 1
}
```

#### Response

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

### paywall_get_boost_element_quote

Get a quote for boosting an element.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_boost_element_quote",
  "params": {
    "element_id": "123456",
    "amount": 1000
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "amount_sats": 100,
    "description": "Boost element 123456"
  },
  "id": 1
}
```

### paywall_boost_element

Boosts an element through the paywall system.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_boost_element",
  "params": {
    "element_id": "123456",
    "amount": 1000,
    "payment_hash": "abcdef123456"
  },
  "id": 1
}
```

#### Response

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

### add_element_comment

Adds a comment to an element.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_element_comment",
  "params": {
    "element_id": "123456",
    "content": "This is a great place!",
    "user_id": "789"
  },
  "id": 1
}
```

#### Response

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

### paywall_get_add_element_comment_quote

Get a quote for adding a comment.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_get_add_element_comment_quote",
  "params": {
    "element_id": "123456",
    "content": "This is a great place!",
    "user_id": "789"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "amount_sats": 10,
    "description": "Add comment to element 123456"
  },
  "id": 1
}
```

### paywall_add_element_comment

Adds a comment to an element through the paywall system.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "paywall_add_element_comment",
  "params": {
    "element_id": "123456",
    "content": "This is a great place!",
    "user_id": "789",
    "payment_hash": "abcdef123456"
  },
  "id": 1
}
```

#### Response

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

### generate_element_issues

Generates issues for elements.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_issues",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "issues_generated": 10
  },
  "id": 1
}
```

### sync_elements

Synchronizes elements from an external source.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "sync_elements",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "elements_synced": 100
  },
  "id": 1
}
```

### generate_element_icons

Generates icons for elements.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_icons",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "icons_generated": 50
  },
  "id": 1
}
```

### generate_element_categories

Generates categories for elements.

**Required Admin Action**: `element_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_element_categories",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "categories_generated": 20
  },
  "id": 1
}
```

### get_element_issues

Get issues associated with elements.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_element_issues",
  "params": {
    "element_id": 123456
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "id": 42,
      "element_id": 123456,
      "issue_type": "closed",
      "comment": "This location is permanently closed",
      "resolved": true,
      "resolution_comment": "Updated element information"
    }
  ],
  "id": 1
}
```
