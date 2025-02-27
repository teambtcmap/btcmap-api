
# Search Methods

This document describes the available RPC methods for searching.

## Table of Contents

- [search](#search) - Search for elements, areas, or users

## Methods

### search

Searches for elements, areas, or users.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "search",
  "params": {
    "query": "coffee",
    "type": "element",
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
        "name": "Bitcoin Coffee",
        "osm_type": "node",
        "osm_id": 123456,
        "tags": {
          "name": "Bitcoin Coffee",
          "amenity": "cafe",
          "currency:XBT": "yes"
        }
      }
    ],
    "total_count": 1
  },
  "id": 1
}
```
