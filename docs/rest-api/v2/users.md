
# Users API (v2)

Endpoints for retrieving user data in the v2 API.

## Get Users

Retrieves a list of users.

```
GET /v2/users
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| limit     | int  | Maximum number of users to return |
| offset    | int  | Number of users to skip |

### Example Response

```json
{
  "users": [
    {
      "id": "123",
      "username": "exampleuser",
      "created_at": "2023-01-01T00:00:00Z"
    }
  ],
  "total": 50
}
```

## Get User by ID

Retrieves a specific user by their ID.

```
GET /v2/users/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | string | The ID of the user |

### Example Response

```json
{
  "id": "123",
  "username": "exampleuser",
  "created_at": "2023-01-01T00:00:00Z"
}
```
