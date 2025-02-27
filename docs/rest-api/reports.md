
# Reports API

The Reports API provides access to user reports about elements on the map.

## Endpoints

### GET /v3/reports

Retrieve a list of reports that have been updated since a specified time.

#### Query Parameters

- `updated_since` (required): RFC3339 formatted datetime string
- `limit` (required): Maximum number of reports to return

#### Example Request

```
GET /v3/reports?updated_since=2023-01-01T00:00:00Z&limit=10
```

#### Example Response

```json
[
  {
    "id": 123,
    "element_id": 456,
    "user_id": 789,
    "text": "This place is permanently closed",
    "updated_at": "2023-02-15T14:30:45Z"
  },
  {
    "id": 124,
    "element_id": 457,
    "user_id": 790,
    "text": "Incorrect opening hours",
    "updated_at": "2023-02-16T09:12:33Z"
  }
]
```

### GET /v3/reports/{id}

Retrieve a specific report by its ID.

#### Path Parameters

- `id`: The unique identifier of the report

#### Example Request

```
GET /v3/reports/123
```

#### Example Response

```json
{
  "id": 123,
  "element_id": 456,
  "user_id": 789,
  "text": "This place is permanently closed",
  "updated_at": "2023-02-15T14:30:45Z"
}
```
