# Search Methods

This document describes the available RPC methods for searching.

## Available Methods

- [search](#search) - Search for elements, areas, or users

## Search

Searches for elements, areas, users, and other entities.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "search",
  "params": {
    "query": "search term",
    "types": ["elements", "areas", "users"],
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
    "elements": [
      {
        "id": "element_id",
        "name": "Element Name",
        "description": "Element Description",
        "relevance_score": 0.95
      }
    ],
    "areas": [
      {
        "id": "area_id",
        "name": "Area Name",
        "url_alias": "area-name",
        "relevance_score": 0.85
      }
    ],
    "users": [
      {
        "id": "user_id",
        "name": "User Name",
        "relevance_score": 0.75
      }
    ]
  },
  "id": 1
}