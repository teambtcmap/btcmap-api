
# Reports API (v2)

Endpoints for retrieving report data in the v2 API.

## Get Reports

Retrieves a list of reports.

```
GET /v2/reports
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| limit     | int  | Maximum number of reports to return |
| offset    | int  | Number of reports to skip |

### Example Response

```json
{
  "reports": [
    {
      "id": "1",
      "element_id": "123",
      "user_id": "456",
      "reason": "Element needs verification",
      "created_at": "2023-01-01T00:00:00Z"
    }
  ],
  "total": 20
}
```

## Get Report by ID

Retrieves a specific report by its ID.

```
GET /v2/reports/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | string | The ID of the report |

### Example Response

```json
{
  "id": "1",
  "element_id": "123",
  "user_id": "456",
  "reason": "Element needs verification",
  "created_at": "2023-01-01T00:00:00Z"
}
```
