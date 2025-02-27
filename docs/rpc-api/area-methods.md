
# Area RPC Methods

This page documents all RPC methods related to areas.

## AddArea

Adds a new area. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "add_area",
  "params": {
    "name": "Area Name",
    "url_alias": "area-name",
    "coordinates": {
      "type": "Polygon",
      "coordinates": [[[lon1, lat1], [lon2, lat2], [lon3, lat3], [lon1, lat1]]]
    }
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
    "url_alias": "area-name"
  },
  "id": 1
}
```

## GetArea

Retrieves a specific area by its ID or URL alias.

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

OR

```json
{
  "jsonrpc": "2.0",
  "method": "get_area",
  "params": {
    "url_alias": "area-name"
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
    "url_alias": "area-name",
    "coordinates": {
      "type": "Polygon",
      "coordinates": [[[lon1, lat1], [lon2, lat2], [lon3, lat3], [lon1, lat1]]]
    },
    "tags": ["tag1", "tag2"],
    "icon": "icon_url",
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z"
  },
  "id": 1
}
```

## SetAreaTag

Adds a tag to an area. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_area_tag",
  "params": {
    "area_id": "area_id",
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

## RemoveAreaTag

Removes a tag from an area. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_area_tag",
  "params": {
    "area_id": "area_id",
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

## SetAreaIcon

Sets an icon for an area.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_area_icon",
  "params": {
    "area_id": "area_id",
    "icon": "base64_encoded_icon"
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
    "icon_url": "icon_url"
  },
  "id": 1
}
```

## RemoveArea

Removes an area. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_area",
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
    "success": true
  },
  "id": 1
}
```

## GetTrendingCountries

Retrieves a list of trending countries.

### Request

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

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "countries": [
      {
        "code": "US",
        "name": "United States",
        "element_count": 1000,
        "trend_score": 95
      },
      {
        "code": "DE",
        "name": "Germany",
        "element_count": 500,
        "trend_score": 80
      }
    ]
  },
  "id": 1
}
```

## GetMostCommentedCountries

Retrieves a list of countries with the most comments.

### Request

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

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "countries": [
      {
        "code": "US",
        "name": "United States",
        "comment_count": 2000
      },
      {
        "code": "DE",
        "name": "Germany",
        "comment_count": 1500
      }
    ]
  },
  "id": 1
}
```

## GetTrendingCommunities

Retrieves a list of trending communities.

### Request

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

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "communities": [
      {
        "id": "area_id",
        "name": "Community Name",
        "element_count": 100,
        "trend_score": 90
      }
    ]
  },
  "id": 1
}
```

## GenerateAreasElementsMapping

Generates a mapping between areas and elements. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_areas_elements_mapping",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "mappings_generated": 500
  },
  "id": 1
}
```

## GenerateReports

Generates reports. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "generate_reports",
  "params": {},
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "reports_generated": 20
  },
  "id": 1
}
```

## GetAreaDashboard

Retrieves dashboard data for an area.

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
    "area": {
      "id": "area_id",
      "name": "Area Name"
    },
    "stats": {
      "element_count": 100,
      "comment_count": 50,
      "active_users": 30,
      "recent_activity": 20
    },
    "trending_elements": [
      {
        "id": "element_id",
        "name": "Element Name",
        "trend_score": 95
      }
    ]
  },
  "id": 1
}
```
