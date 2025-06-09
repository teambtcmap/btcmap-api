
# Reports API (v3)

Endpoints for retrieving report data in the v3 API. This version provides enhanced filtering and more detailed report information.

## Get Reports

Retrieves a list of reports with enhanced filtering.

```
GET /v3/reports
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return reports updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of reports to return |
| element_id     | int    | Filter reports by element ID |
| status         | string | Filter by report status (open, closed, all) |

### Example Response

```json
[
  {
    "id": 1,
    "element_id": 123,
    "user_id": 456,
    "reason": "Element needs verification",
    "description": "I visited this place and it seems to be closed permanently",
    "status": "open",
    "created_at": "2023-01-01T00:00:00Z",
    "updated_at": "2023-01-01T00:00:00Z",
    "resolved_at": null,
    "resolved_by": null
  }
]
```

## Get Report by ID

Retrieves a specific report by its ID with enhanced details.

```
GET /v3/reports/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the report |

### Example Response

```json
{
  "id": 1,
  "element_id": 123,
  "user_id": 456,
  "reason": "Element needs verification",
  "description": "I visited this place and it seems to be closed permanently",
  "status": "open",
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z",
  "resolved_at": null,
  "resolved_by": null,
  "comments": [
    {
      "id": 789,
      "user_id": 789,
      "content": "I'll check this place next week",
      "created_at": "2023-01-02T00:00:00Z"
    }
  ]
}
```
