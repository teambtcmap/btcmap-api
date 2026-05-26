# Place Issues REST API (v4)

This document describes the endpoint for fetching place issues in REST API v4.

## Get Place Issues

Retrieves issues for places within a specified area, ordered by severity.

```bash
curl 'https://api.btcmap.org/v4/place-issues?area_id=120'
```

#### Parameters

| Parameter | Type | Example | Default | Description |
|-----------|------|---------|---------|-------------|
| `area_id` | Integer | `120` | - | **Required**. Area ID to fetch issues for. |
| `limit` | Integer | `50` | `50` | Maximum number of issues to return. |
| `offset` | Integer | `0` | `0` | Number of issues to skip for pagination. |

#### Response

```json
{
  "total_issues": 123,
  "requested_issues": [
    {
      "element_osm_type": "node",
      "element_osm_id": 123456789,
      "element_name": "Coffee Shop",
      "issue_code": "wrong_location"
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `total_issues` | Number | Total count of issues in the area (excludes outdated/not_verified for non-global areas). |
| `requested_issues` | Array | List of issues matching the query. |
| `element_osm_type` | String | OSM element type (`node`, `way`, or `relation`). |
| `element_osm_id` | Number | OSM element ID. |
| `element_name` | String | Name of the place from OSM tags. |
| `issue_code` | String | Issue type code (e.g., `wrong_location`, `outdated`, `not_verified`). |

#### Special Behavior

- For `area_id = 662` (global/unfiltered): All issue types are returned including `outdated`, `outdated_soon`, and `not_verified`.
- For all other areas: Issues with codes `outdated`, `outdated_soon`, and `not_verified` are filtered out to show only actionable issues.
- Issues are ordered by severity (highest first).

#### Examples

##### Get Issues for a Community Area

```bash
curl 'https://api.btcmap.org/v4/place-issues?area_id=120'
```

```json
{
  "total_issues": 5,
  "requested_issues": [
    {
      "element_osm_type": "node",
      "element_osm_id": 987654321,
      "element_name": "Bitcoin ATM",
      "issue_code": "wrong_location"
    },
    {
      "element_osm_type": "way",
      "element_osm_id": 456789123,
      "element_name": "Satoshi's Pub",
      "issue_code": "permanently_closed"
    }
  ]
}
```

##### Paginate Issues

```bash
curl 'https://api.btcmap.org/v4/place-issues?area_id=120&limit=10&offset=20'
```
