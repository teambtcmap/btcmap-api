# Users REST API (v4)

This document describes the endpoints for interacting with users in REST API v4.

## Available Endpoints

- [Get Authenticated User](#get-authenticated-user)

### Get Authenticated User

Returns the currently authenticated user's information. Requires a valid Bearer token in the Authorization header.

#### Example Request

```bash
curl https://api.btcmap.org/v4/users/me \
  -H "Authorization: Bearer <your-token>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns user information |
| 401  | Unauthorized - Missing or invalid token |

##### Example Response (200 OK)

```json
{
  "id": 123,
  "name": "satoshi",
  "roles": ["user", "admin"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username |
| roles | Array  | List of user roles (e.g., "user", "admin", "root") |
