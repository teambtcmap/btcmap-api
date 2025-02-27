
# Users API

The Users API provides access to user information.

## Endpoints

### GET /v3/users

Retrieve a list of users that have been updated since a specified time.

#### Query Parameters

- `updated_since` (required): RFC3339 formatted datetime string
- `limit` (required): Maximum number of users to return

#### Example Request

```
GET /v3/users?updated_since=2023-01-01T00:00:00Z&limit=10
```

#### Example Response

```json
[
  {
    "id": 789,
    "osm_data": {
      "id": 789,
      "display_name": "JohnDoe",
      "account_created": "2020-01-15T08:12:45Z"
    },
    "tags": {
      "experience": "contributor",
      "interests": ["bitcoin", "mapping"]
    },
    "updated_at": "2023-02-15T14:30:45Z"
  },
  {
    "id": 790,
    "osm_data": {
      "id": 790,
      "display_name": "JaneDoe",
      "account_created": "2021-03-22T10:45:12Z"
    },
    "tags": {
      "experience": "moderator",
      "regions": ["Europe", "Asia"]
    },
    "updated_at": "2023-02-16T09:12:33Z"
  }
]
```

### GET /v3/users/{id}

Retrieve a specific user by their ID.

#### Path Parameters

- `id`: The unique identifier of the user

#### Example Request

```
GET /v3/users/789
```

#### Example Response

```json
{
  "id": 789,
  "osm_data": {
    "id": 789,
    "display_name": "JohnDoe",
    "account_created": "2020-01-15T08:12:45Z"
  },
  "tags": {
    "experience": "contributor",
    "interests": ["bitcoin", "mapping"]
  },
  "updated_at": "2023-02-15T14:30:45Z"
}
```
