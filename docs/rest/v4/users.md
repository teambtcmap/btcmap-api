# Users REST API (v4)

This document describes the endpoints for interacting with users in REST API v4.

## Available Endpoints

- [Get Authenticated User](#get-authenticated-user)
- [Create User](#create-user)
- [Create Token](#create-token)
- [Change Password](#change-password)
- [Update Username](#update-username)
- [Get Linked Nostr Identity](#get-linked-nostr-identity)
- [Link Nostr Identity](#link-nostr-identity)
- [Unlink Nostr Identity](#unlink-nostr-identity)

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
  "saved_places": [{"id": 1, "name": "Bitcoin Cafe"}],
  "saved_areas": [{"id": 2, "name": "Downtown District"}],
  "npub": "npub1..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username |
| roles | Array  | List of user roles (e.g., "user", "admin", "root") |
| saved_places | Array | List of saved places with `id` and `name` fields |
| saved_areas | Array | List of saved areas with `id` and `name` fields |
| npub  | String \| null | Bech32 npub of the linked Nostr identity, or `null` if none is linked |

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

Returns the authenticated user (same shape as [Get Authenticated User](#get-authenticated-user)). Note that `saved_places` and `saved_areas` are always returned empty here.

```json
{
  "id": 124,
  "name": "newSatoshi",
  "roles": ["user"],
  "saved_places": [],
  "saved_areas": [],
  "npub": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Updated username |
| roles | Array  | List of user roles |
| saved_places | Array | Always empty on this endpoint |
| saved_areas | Array | Always empty on this endpoint |
| npub  | String \| null | Bech32 npub of the linked Nostr identity, or `null` if none is linked |

### Get Linked Nostr Identity

Returns the Nostr pubkey currently linked to the authenticated account, or `null` if none is linked. Requires a valid Bearer token. This is the same `npub` exposed on [Get Authenticated User](#get-authenticated-user), offered as a dedicated sub-resource so a client can poll just the link state.

#### Example Request

```bash
curl https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns the linked npub (or null) |
| 401  | Unauthorized - Missing or invalid token |

##### Example Response (200 OK)

```json
{
  "npub": "npub1..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| npub  | String \| null | Bech32 npub of the linked Nostr identity, or `null` if none is linked |

### Link Nostr Identity

Links (or replaces) the Nostr pubkey on the authenticated account. This requires **two** credentials at once:

- `Authorization: Bearer <token>` — identifies the account being modified.
- `X-Nostr-Authorization: Nostr <base64-event>` — a NIP-98 event proving control of the pubkey being linked.

The two cannot share the `Authorization` header, so the NIP-98 proof is carried on the dedicated `X-Nostr-Authorization` header. The request body is empty. The proof event must sign `u = <api-base-url>/v4/users/me/nostr` with method `PUT` (the `u`/`method` are matched against the server's configured base URL and the actual request method — both are case-sensitive).

#### Example Request

```bash
curl -X PUT https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>" \
  -H "X-Nostr-Authorization: Nostr <base64-encoded-nip98-event>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Pubkey linked (or already linked to this account) |
| 400  | Bad Request - The npub is already linked to a different account |
| 401  | Unauthorized - Missing/invalid Bearer token, or missing/invalid NIP-98 proof |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "npub": "npub1..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| npub  | String | Bech32 npub now linked to the account |

### Unlink Nostr Identity

Clears the Nostr pubkey linked to the authenticated account. Requires a valid Bearer token only — removing your own link needs no NIP-98 proof. Idempotent: succeeds with `npub: null` even if nothing was linked.

#### Example Request

```bash
curl -X DELETE https://api.btcmap.org/v4/users/me/nostr \
  -H "Authorization: Bearer <your-token>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Link cleared (or already absent) |
| 401  | Unauthorized - Missing or invalid token |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "npub": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| npub  | null | Always `null` after unlinking |