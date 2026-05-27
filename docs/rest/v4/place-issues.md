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
      "issue_code": "missing_icon"
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
| `issue_code` | String | Issue type code (`outdated`, `outdated_soon`, `not_verified`, `missing_icon`, `invalid_tag_value:survey:date`, `invalid_tag_value:check_date`, `invalid_tag_value:check_date:currency:XBT`, `misspelled_tag_name:payment:lighting`, `misspelled_tag_name:lightning_contacless`, `misspelled_tag_name:lighting_contactless`, `unknown`). |

#### Special Behavior

- For `area_id = 662` (global/unfiltered): All issue types are returned.
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
      "issue_code": "missing_icon"
    },
    {
      "element_osm_type": "way",
      "element_osm_id": 456789123,
      "element_name": "Satoshi's Pub",
      "issue_code": "outdated"
    }
  ]
}
```

##### Paginate Issues

```bash
curl 'https://api.btcmap.org/v4/place-issues?area_id=120&limit=10&offset=20'
```

## Get Place Issue by ID

Retrieves a single place issue by its ID.

```bash
curl 'https://api.btcmap.org/v4/place-issues/123'
```

#### Path Parameters

| Parameter | Type | Example | Description |
|-----------|------|---------|-------------|
| `id` | Integer | `123` | **Required**. Unique identifier of the place issue. |

#### Response

```json
{
  "id": 123,
  "place_id": 456,
  "code": "outdated",
  "severity": 3,
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z",
  "deleted_at": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | Number | Unique issue identifier. |
| `place_id` | Number | Associated place/element ID. |
| `code` | String | Issue type code. See Issue Codes table below for all valid values. |
| `severity` | Number | Severity level (higher = more severe). |
| `created_at` | String | ISO 8601 timestamp when the issue was created. |
| `updated_at` | String | ISO 8601 timestamp when the issue was last updated. |
| `deleted_at` | String | ISO 8601 timestamp when the issue was soft-deleted, or `null` if active. |

#### Issue Codes

| Code | Description |
|------|-------------|
| `outdated` | The place information is outdated. |
| `outdated_soon` | The place will be outdated soon, it's time to bump check date. |
| `not_verified` | The place has not been verified by an editor. |
| `missing_icon` | The place is missing an icon in the app. |
| `invalid_tag_value:survey:date` | The `survey:date` tag is not formatted properly. |
| `invalid_tag_value:check_date` | The `check_date` tag is not formatted properly. |
| `invalid_tag_value:check_date:currency:XBT` | The `check_date:currency:XBT` tag is not formatted properly. |
| `misspelled_tag_name:payment:lighting` | The `payment:lighting` tag is misspelled. |
| `misspelled_tag_name:lightning_contacless` | The `payment:lightning_contacless` tag is misspelled. |
| `misspelled_tag_name:lighting_contactless` | The `payment:lighting_contactless` tag is misspelled. |
| `unknown` | An unknown issue type. |
