# Users REST API (v4)

This document describes the endpoints for interacting with users in REST API v4.

## Available Endpoints

- [Get Authenticated User](#get-authenticated-user)
- [Create User](#create-user)
- [Search Users](#search-users)
- [Update User](#update-user)
- [Create Token](#create-token)
- [Change Password](#change-password)
- [Update Username](#update-username)
- [Get Linked Nostr Identity](#get-linked-nostr-identity)
- [Link Nostr Identity](#link-nostr-identity)
- [Unlink Nostr Identity](#unlink-nostr-identity)
- [List My Place Images](#list-my-place-images)
- [List My Events](#list-my-events)

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
  "geofence": [],
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
| geofence | Array of Numbers | Area IDs the user is restricted to when acting as an event manager. Empty means unrestricted. |
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

### Search Users

Admin and root only user lookup. Returns users whose `name` contains `query` (case-insensitive substring), ordered by name. Soft-deleted users are excluded and `%`/`_` in `query` are matched literally. An empty `query` lists users up to `limit`.

#### Example Request

```bash
curl 'https://api.btcmap.org/v4/users?query=na&limit=50' \
  -H "Authorization: Bearer <your-token>"
```

#### Query Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| query | String | Yes | Case-insensitive substring to match against usernames. An empty value lists users up to `limit`. |
| limit | Number | No | Maximum number of users to return. Defaults to 100 and must be between 1 and 1000. |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns matching users (possibly empty) |
| 400  | Bad Request - `limit` is out of range |
| 401  | Unauthorized - Missing or invalid token |
| 403  | Forbidden - Caller is not an admin or root |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
[
  {
    "id": 124,
    "name": "natinfosec",
    "roles": ["user"],
    "created_at": "2024-01-01T00:00:00Z",
    "geofence": [],
    "npub": "npub1..."
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username |
| roles | Array  | List of user roles |
| created_at | String | RFC 3339 creation timestamp |
| geofence | Array of Numbers | Area IDs the user is restricted to when acting as an event manager. Empty means unrestricted. |
| npub  | String | Bech32 npub of the linked Nostr identity; omitted when none is linked |

### Update User

Root and admin only partial update of a user's `roles` and/or `geofence`. Both fields are optional; omitted fields are left unchanged. Returns the updated user in the same shape as [Search Users](#search-users).

The applied policy:

- **Root** may update the roles (up to, but never `root`) and geofence of any non-root user, and may update its own geofence, but may never change its own roles or touch another root.
- **Admin** may add or remove `event_manager` / `area_manager` for non-admin, non-root users, and may set the geofence of any target that is not an admin or a root (including their own). Admins can never change their own roles.

#### Example Request

```bash
curl -X PATCH https://api.btcmap.org/v4/users/124 \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"roles": ["user", "event_manager"], "geofence": [3, 7]}'
```

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| roles | Array of Strings | No | Replacement role set. Omit to leave roles untouched. |
| geofence | Array of Numbers | No | Replacement geofence (area IDs). Omit to leave the geofence untouched. |

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns the updated user |
| 400  | Bad Request - Neither field provided, or an unknown role |
| 401  | Unauthorized - Missing or invalid token |
| 403  | Forbidden - Caller is not allowed to make this change |
| 404  | Not Found - No user with the given id |
| 500  | Internal Server Error - Database error |

##### Example Response (200 OK)

```json
{
  "id": 124,
  "name": "natinfosec",
  "roles": ["user", "event_manager"],
  "created_at": "2024-01-01T00:00:00Z",
  "geofence": [3, 7],
  "npub": "npub1..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| id    | Number | User ID |
| name  | String | Username |
| roles | Array  | Updated list of user roles |
| created_at | String | RFC 3339 creation timestamp |
| geofence | Array of Numbers | Updated area IDs the user is restricted to when acting as an event manager. Empty means unrestricted. |
| npub  | String | Bech32 npub of the linked Nostr identity; omitted when none is linked |

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
  "token": "550e8400-e29b-41d4-a716-446655440000",
  "user": {
    "id": 124,
    "name": "satoshi",
    "roles": ["user"],
    "saved_places": [{"id": 1, "name": "Bitcoin Cafe"}],
    "saved_areas": [{"id": 2, "name": "Downtown District"}],
    "geofence": [],
    "npub": null
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| token | String | New authentication token (UUID v4) |
| user | Object | Authenticated user (same shape as [Get Authenticated User](#get-authenticated-user)) |

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
  "geofence": [],
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
| geofence | Array of Numbers | Area IDs the user is restricted to when acting as an event manager. Empty means unrestricted. |
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

Here `<api-base-url>` is the server's configured `BTCMAP_API_BASE_URL`, not the host from the request headers. See [Server Configuration (NIP-98)](auth.md#server-configuration-nip-98) for why this must match the public origin.

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

### List My Place Images

Returns the place images uploaded by the authenticated user, across all places,
newest first. Requires a valid Bearer token. Use it to find an accidentally
uploaded photo and delete it via [Delete Place Image](place-images.md#delete-place-image).

#### Example Request

```bash
curl https://api.btcmap.org/v4/users/me/place-images \
  -H "Authorization: Bearer <your-token>"
```

#### Response

Returns an array of place image items. See
[List My Place Images](place-images.md#list-my-place-images) in the Place Images
documentation for the response shape and errors.

| Code | Description |
|------|-------------|
| 200  | Success - Returns the user's uploaded images (possibly empty) |
| 401  | Unauthorized - Missing or invalid token |
| 500  | Internal Server Error - Database error |

### List My Events

Returns the events submitted by the authenticated user, across all review
statuses, newest first. Requires a valid Bearer token. Use it to track the
`pending`/`live`/`rejected` state of submissions.

#### Example Request

```bash
curl https://api.btcmap.org/v4/users/me/events \
  -H "Authorization: Bearer <your-token>"
```

#### Response

Returns an array of event items. See
[Get My Submitted Events](events.md#get-my-submitted-events) in the Events
documentation for the response shape and errors.

| Code | Description |
|------|-------------|
| 200  | Success - Returns the user's submitted events (possibly empty) |
| 401  | Unauthorized - Missing or invalid token |
| 500  | Internal Server Error - Database error |