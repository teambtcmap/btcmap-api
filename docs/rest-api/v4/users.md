# Users API (v4)

The Users API allows you to retrieve information about users of the BTCMap platform.

## Endpoints

### Get Users List

```
GET /v4/users
```

Retrieves a list of users that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | Required. Filter users updated since this time (RFC3339 format). |
| `limit` | Integer | Required. Limit the number of users returned. |

#### Response

```json
[
  {
    "id": 123,
    "osm_data": {
      "id": 123,
      "display_name": "username",
      "account_created": "2020-01-01T00:00:00Z"
    },
    "tags": {
      "role": "contributor"
    },
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

### Get User by ID

```
GET /v4/users/{id}
```

Retrieves a specific user by their ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | The user ID |

#### Response

```json
{
  "id": 123,
  "osm_data": {
    "id": 123,
    "display_name": "username",
    "account_created": "2020-01-01T00:00:00Z"
  },
  "tags": {
    "role": "contributor"
  },
  "updated_at": "2023-01-15T00:00:00Z"
}
```

## Examples

### Get all users updated since January 2023 with a limit of 10

```
GET /v4/users?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get a specific user

```
GET /v4/users/123
```