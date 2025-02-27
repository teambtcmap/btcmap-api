# Element Methods

This document describes the available RPC methods for interacting with elements (locations that accept Bitcoin).

## Methods

### get_elements

Retrieves elements based on query parameters.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_elements",
  "params": {
    "updated_since": "2023-01-01T00:00:00Z",
    "limit": 10
  },
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

### get_element_by_id

Retrieves a specific element by its ID.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_element_by_id",
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

### update_element

Updates an element's tags.

**Required Role**: `element:edit`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "update_element",
  "params": {
    "password": "your_admin_password",
    "id": 123456,
    "tags": {
      "name": "Bitcoin Coffee Shop",
      "amenity": "cafe",
      "currency:XBT": "yes",
      "opening_hours": "Mo-Fr 08:00-17:00"
    }
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
    "element": {
      "id": 123456,
      "osm_type": "node",
      "osm_id": 123456,
      "tags": {
        "name": "Bitcoin Coffee Shop",
        "amenity": "cafe",
        "currency:XBT": "yes",
        "opening_hours": "Mo-Fr 08:00-17:00"
      }
    }
  },
  "id": 1
}
```

### remove_element

Removes an element from the database.

**Required Role**: `element:remove`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_element",
  "params": {
    "password": "your_admin_password",
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
    "success": true
  },
  "id": 1
}
```

### report_element_issue

Reports an issue with an element.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "report_element_issue",
  "params": {
    "element_id": 123456,
    "issue_type": "closed",
    "comment": "This location is permanently closed"
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
    "issue_id": 42
  },
  "id": 1
}
```

### resolve_element_issue

Resolves a reported issue with an element.

**Required Role**: `issue:resolve`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "resolve_element_issue",
  "params": {
    "password": "your_admin_password",
    "issue_id": 42,
    "resolution_comment": "Updated element information"
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

## Admin Methods

### update_element_tags
Updates the tags of a specific element.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | String | Yes | The element ID |
| `tags` | Object | Yes | The updated tags |

**Response**:

The updated element object.

### delete_element
Marks an element as deleted.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | String | Yes | The element ID |

**Response**:

```json
{
  "success": true
}
```

### SetElementTag
Adds a tag to an element.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `tag` | String | Yes | The tag name |

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### RemoveElementTag
Removes a tag from an element.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `tag` | String | Yes | The tag name |

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

### BoostElement
Boosts an element.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `amount` | Integer | Yes | The boost amount |

**Response**:

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

### PaywallBoostElement
Boosts an element through the paywall system.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `amount` | Integer | Yes | The boost amount |
| `payment_hash` | String | Yes | The payment hash |

**Response**:

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

### AddElementComment
Adds a comment to an element.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `content` | String | Yes | The comment content |
| `user_id` | String | Yes | The user ID |

**Response**:

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

### PaywallAddElementComment
Adds a comment to an element through the paywall system.

**Required Role**: `element_admin`

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |
| `content` | String | Yes | The comment content |
| `user_id` | String | Yes | The user ID |
| `payment_hash` | String | Yes | The payment hash |

**Response**:

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

### GenerateElementIssues
Generates issues for elements.

**Required Role**: `element_admin`

**Parameters**:
None

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "issues_generated": 10
  },
  "id": 1
}
```

### SyncElements
Synchronizes elements from an external source.

**Required Role**: `element_admin`

**Parameters**:
None

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "elements_synced": 100
  },
  "id": 1
}
```

### GenerateElementIcons
Generates icons for elements.

**Required Role**: `element_admin`

**Parameters**:
None

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "icons_generated": 50
  },
  "id": 1
}
```

### GenerateElementCategories
Generates categories for elements.

**Required Role**: `element_admin`

**Parameters**:
None

**Response**:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "categories_generated": 20
  },
  "id": 1
}