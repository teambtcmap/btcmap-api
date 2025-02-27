# Element Methods

Methods for managing map elements in the BTCMap platform.

## Public Methods

### get_element

Retrieves a specific element by its ID.

**Required Role**: None (Public)

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | String | Yes | The element ID |

**Response**:

```json
{
  "id": "123",
  "type": "node",
  "tags": {
    "name": "Example Shop",
    "amenity": "cafe"
  },
  "geometry": {
    "type": "Point",
    "coordinates": [13.37, 42.0]
  },
  "updated_at": "2023-01-15T00:00:00Z"
}
```

### get_elements_in_area

Retrieves elements within a specified geographic area.

**Required Role**: None (Public)

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `area_id` | String or Integer | Yes | The area ID or URL alias |

**Response**:

Array of element objects.


### GetBoostedElements

Retrieves a list of boosted elements.

**Required Role**: None (Public)

**Parameters**:
None

**Response**:

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

### PaywallGetBoostElementQuote

Gets a quote for boosting an element.

**Required Role**: None (Public)

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `amount` | Integer | Yes | The boost amount |


**Response**:

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

### PaywallGetAddElementCommentQuote

Gets a quote for adding a comment to an element.

**Required Role**: None (Public)

**Parameters**:
None

**Response**:

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

### GetElementIssues

Retrieves issues for an element.

**Required Role**: None (Public)

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `element_id` | String | Yes | The element ID |

**Response**:

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