# Area Methods

This document describes the available RPC methods for interacting with areas.

## Available Methods

- [get_areas](#get_areas) - Retrieve areas based on query parameters
- [get_area_by_id](#get_area_by_id) - Retrieve a specific area by ID
- [get_area_elements](#get_area_elements) - Get elements within an area
- [add_area](#add_area) - Adds a new geographic area.
- [update_area](#update_area) - Updates an existing area.
- [remove_area](#remove_area) - Removes an area from the database.


### <a name="get_areas"></a>get_areas

Retrieves areas based on query parameters.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_areas",
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
    "areas": [
      {
        "id": 123,
        "name": "New York City",
        "url_alias": "nyc",
        "osm_id": 175905,
        "osm_type": "relation",
        "bounds": {
          "min_lon": -74.25909,
          "min_lat": 40.477399,
          "max_lon": -73.700272,
          "max_lat": 40.916178
        }
      }
    ]
  },
  "id": 1
}
```

### <a name="get_area_by_id"></a>get_area_by_id

Retrieves a specific area by its ID.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area_by_id",
  "params": {
    "id": 123
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "area": {
      "id": 123,
      "name": "New York City",
      "url_alias": "nyc",
      "osm_id": 175905,
      "osm_type": "relation",
      "bounds": {
        "min_lon": -74.25909,
        "min_lat": 40.477399,
        "max_lon": -73.700272,
        "max_lat": 40.916178
      }
    }
  },
  "id": 1
}
```

### <a name="get_area_elements"></a>get_area_elements

Retrieves elements within a specific area.

**Required Role**: None

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area_elements",
  "params": {
    "area_id": 123,
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

### <a name="add_area"></a>add_area

Adds a new geographic area.

**Required Role**: `area:add`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_area",
  "params": {
    "password": "your_admin_password",
    "osm_id": 123456,
    "osm_type": "relation",
    "name": "San Francisco",
    "url_alias": "sf"
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
    "area": {
      "id": 789,
      "name": "San Francisco",
      "url_alias": "sf",
      "osm_id": 123456,
      "osm_type": "relation"
    }
  },
  "id": 1
}
```

### <a name="update_area"></a>update_area

Updates an existing area.

**Required Role**: `area:edit`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "update_area",
  "params": {
    "password": "your_admin_password",
    "id": 789,
    "name": "San Francisco Bay Area",
    "url_alias": "sf-bay"
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
    "area": {
      "id": 789,
      "name": "San Francisco Bay Area",
      "url_alias": "sf-bay",
      "osm_id": 123456,
      "osm_type": "relation"
    }
  },
  "id": 1
}
```

### <a name="remove_area"></a>remove_area

Removes an area from the database.

**Required Role**: `area:remove`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_area",
  "params": {
    "password": "your_admin_password",
    "id": 789
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