
# Users API (v3)

Endpoints for retrieving user data in the v3 API. This version provides enhanced filtering and more detailed user information.

## Get Users

Retrieves a list of users with enhanced filtering.

```
GET /v3/users
```

### Query Parameters

| Parameter      | Type   | Description |
|----------------|--------|-------------|
| updated_since  | string | Return users updated since this timestamp (RFC3339 format) |
| limit          | int    | Maximum number of users to return |

### Example Response

```json
[
  {
    "id": 123,
    "osm_data": {
      "id": 987654321,
      "display_name": "ExampleUser",
      "account_created": "2022-01-01T00:00:00Z"
    },
    "tags": {
      "admin": true,
      "moderator": false
    },
    "updated_at": "2023-01-01T00:00:00Z",
    "deleted_at": null
  }
]
```

## Get User by ID

Retrieves a specific user by their ID with enhanced details.

```
GET /v3/users/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | int  | The ID of the user |

### Example Response

```json
{
  "id": 123,
  "osm_data": {
    "id": 987654321,
    "display_name": "ExampleUser",
    "account_created": "2022-01-01T00:00:00Z"
  },
  "tags": {
    "admin": true,
    "moderator": false
  },
  "updated_at": "2023-01-01T00:00:00Z",
  "deleted_at": null
}
```
