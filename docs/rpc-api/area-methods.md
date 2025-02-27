
# Area Methods

This document describes the available RPC methods for interacting with geographic areas.

## Table of Contents

- [add_area](#add_area) - Add a new geographic area
- [get_area](#get_area) - Retrieve a specific area by ID
- [set_area_tag](#set_area_tag) - Set a tag on an area
- [remove_area_tag](#remove_area_tag) - Remove a tag from an area
- [set_area_icon](#set_area_icon) - Set an icon for an area
- [remove_area](#remove_area) - Remove an area
- [get_trending_countries](#get_trending_countries) - Get trending countries
- [get_most_commented_countries](#get_most_commented_countries) - Get most commented countries
- [get_trending_communities](#get_trending_communities) - Get trending communities
- [generate_areas_elements_mapping](#generate_areas_elements_mapping) - Generate mappings between areas and elements
- [generate_reports](#generate_reports) - Generate reports for areas
- [get_area_dashboard](#get_area_dashboard) - Get dashboard data for an area

## Methods

### add_area

Adds a new geographic area.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_area",
  "params": {
    "name": "City Center",
    "polygon": [
      {"lat": 37.7749, "lon": -122.4194},
      {"lat": 37.7749, "lon": -122.4184},
      {"lat": 37.7739, "lon": -122.4184},
      {"lat": 37.7739, "lon": -122.4194},
      {"lat": 37.7749, "lon": -122.4194}
    ],
    "type": "neighborhood"
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "area_id": 123
  },
  "id": 1
}
```

### get_area

Retrieves a specific area by ID.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area",
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
    "id": 123,
    "name": "City Center",
    "polygon": [
      {"lat": 37.7749, "lon": -122.4194},
      {"lat": 37.7749, "lon": -122.4184},
      {"lat": 37.7739, "lon": -122.4184},
      {"lat": 37.7739, "lon": -122.4194},
      {"lat": 37.7749, "lon": -122.4194}
    ],
    "type": "neighborhood",
    "created_at": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

### set_area_tag

Sets a tag on an area.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_area_tag",
  "params": {
    "area_id": 123,
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

### remove_area_tag

Removes a tag from an area.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_area_tag",
  "params": {
    "area_id": 123,
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

### set_area_icon

Sets an icon for an area.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_area_icon",
  "params": {
    "area_id": 123,
    "icon_url": "https://example.com/icon.png"
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

### remove_area

Removes an area.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_area",
  "params": {
    "area_id": 123
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

### get_trending_countries

Gets trending countries.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_trending_countries",
  "params": {
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
    "countries": [
      {
        "id": 123,
        "name": "United States",
        "element_count": 500,
        "trend_score": 95.5
      }
    ]
  },
  "id": 1
}
```

### get_most_commented_countries

Gets most commented countries.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_most_commented_countries",
  "params": {
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
    "countries": [
      {
        "id": 123,
        "name": "United States",
        "comment_count": 350
      }
    ]
  },
  "id": 1
}
```

### get_trending_communities

Gets trending communities.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_trending_communities",
  "params": {
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
    "communities": [
      {
        "id": 456,
        "name": "Austin",
        "element_count": 120,
        "trend_score": 87.3
      }
    ]
  },
  "id": 1
}
```

### generate_areas_elements_mapping

Generates mappings between areas and elements.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_areas_elements_mapping",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "mappings_generated": 250
  },
  "id": 1
}
```

### generate_reports

Generates reports for areas.

**Required Admin Action**: `area_admin`

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_reports",
  "params": {},
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "reports_generated": 15
  },
  "id": 1
}
```

### get_area_dashboard

Gets dashboard data for an area.

**Required Admin Action**: None (publicly accessible)

#### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_area_dashboard",
  "params": {
    "area_id": 123
  },
  "id": 1
}
```

#### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "area_id": 123,
    "name": "City Center",
    "element_count": 120,
    "comment_count": 75,
    "recent_activity": [
      {
        "type": "element_added",
        "timestamp": "2023-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```
