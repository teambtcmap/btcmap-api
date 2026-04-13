# Users REST API (v4)

This document describes the endpoints for interacting with users in REST API v4.

## Available Endpoints

- [Get Authenticated User](#get-authenticated-user)
- [Create User](#create-user)
- [Create Token](#create-token)
- [Change Password](#change-password)
- [Update Username](#update-username)
- [Get Nostr Identity](#get-nostr-identity)
- [Link/Update Nostr Identity](#linkupdate-nostr-identity)
- [Remove Nostr Identity](#remove-nostr-identity)

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
  "roles": ["user", "admin"],
  "npub": "abc123def456..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username |
| roles | Array  | List of user roles (e.g., "user", "admin", "root") |
| npub  | String or null | Linked Nostr public key (hex-encoded), omitted if not linked |

### Create User

Creates a new user account. The `name` field is optional - if not provided, a random name will be generated.

#### Example Request

```bash
# With random generated name
curl -X POST https://api.btcmap.org/v4/users \
  -H "Content-Type: application/json" \
  -d '{"password": "SuperSecurePassword"}'

# With custom name
curl -X POST https://api.btcmap.org/v4/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Satoshi", "password": "SuperSecurePassword"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name  | String | No | Username. If not provided, a random name will be generated |
| password | String | Yes | User's password |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - User created |
| 400  | Bad Request - Invalid input (e.g., empty password) |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "id": 124,
  "name": "Satoshi",
  "roles": ["user"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username (either provided or generated) |
| roles | Array  | List of user roles (default: ["user"]) |

### Create Token

Creates a new authentication token for the user. Authenticates via password in the Authorization header.

#### Example Request

```bash
curl -X POST https://api.btcmap.org/v4/users/satoshi/tokens \
  -H "Authorization: Bearer YourPassword" \
  -H "Content-Type: application/json" \
  -d '{"label": "my-device"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| label | String | No | Label for the token (e.g., "my-device", "mobile") |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Token created |
| 401  | Unauthorized - Invalid credentials |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "token": "550e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Description |
|-------|------|-------------|
| token | String | New authentication token (UUID v4) |

### Change Password

Changes the authenticated user's password. Requires a valid Bearer token.

#### Example Request

```bash
curl -X PUT https://api.btcmap.org/v4/users/me/password \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"old_password": "oldPassword123", "new_password": "newSecurePassword456"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| old_password | String | Yes | User's current password |
| new_password | String | Yes | New password to set |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Password changed |
| 400  | Bad Request - Invalid old password or input error |
| 401  | Unauthorized - Missing or invalid token |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{}
```

### Update Username

Updates the authenticated user's username. Requires a valid Bearer token.

#### Example Request

```bash
curl -X PUT https://api.btcmap.org/v4/users/me/username \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"username": "newSatoshi"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| username | String | Yes | New username to set |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Username updated |
| 401  | Unauthorized - Missing or invalid token |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "id": 124,
  "name": "newSatoshi",
  "roles": ["user"]
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Updated username |
| roles | Array  | List of user roles |

### Get Nostr Identity

Returns the Nostr public key linked to the authenticated user's account. Requires a valid Bearer token.

#### Example Request

```bash
curl https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns linked Nostr pubkey |
| 401  | Unauthorized - Missing or invalid token |

##### Example Response (200 OK)

```json
{
  "npub": "abc123def456..."
}
```

If no Nostr identity is linked, `npub` will be `null`.

| Field | Type | Description |
|-------|------|-------------|
| npub  | String or null | Hex-encoded Nostr public key, or null if not linked |

### Link/Update Nostr Identity

Links or updates the Nostr identity for the authenticated user. The request body must contain a base64-encoded [NIP-98](https://github.com/nostr-protocol/nips/blob/master/98.md) kind 27235 event signed by the Nostr key to link.

The NIP-98 event must have:
- `kind`: 27235
- `u` tag: the exact request URL (e.g., `https://api.btcmap.org/v4/users/me/nostr`)
- `method` tag: `PUT`
- `created_at`: within 60 seconds of server time
- Valid Schnorr signature

Each Nostr pubkey can only be linked to one BTC Map account (and vice versa).

#### Example Request

```bash
curl -X PUT https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"nostr_event": "<base64-encoded NIP-98 event>"}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| nostr_event | String | Yes | Base64-encoded NIP-98 kind 27235 event |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Nostr identity linked/updated |
| 400  | Bad Request - NIP-98 verification failed or pubkey already linked |
| 401  | Unauthorized - Missing or invalid Bearer token |

##### Example Response (200 OK)

```json
{
  "npub": "abc123def456..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| npub  | String | Hex-encoded Nostr public key that was linked |

### Remove Nostr Identity

Removes the Nostr identity linked to the authenticated user's account. Requires a valid Bearer token.

#### Example Request

```bash
curl -X DELETE https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Nostr identity removed |
| 400  | Bad Request - No Nostr identity linked |
| 401  | Unauthorized - Missing or invalid token |

##### Example Response (200 OK)

```json
{
  "npub": null
}