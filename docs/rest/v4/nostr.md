# Nostr REST API (v4)

This document describes the Nostr authentication endpoints in REST API v4.

## Available Endpoints

- [Create Token with Nostr](#create-token-with-nostr)

### Create Token with Nostr

Obtain a BTC Map API Bearer token by authenticating with a [NIP-98](https://github.com/nostr-protocol/nips/blob/master/98.md) signed Nostr event. This is the "login with Nostr" flow -- no password is required.

**Prerequisites**: The Nostr pubkey used to sign the event must already be linked to a BTC Map account via `PUT /v4/users/me/nostr` (see [Users API](users.md#linkupdate-nostr-identity)).

The NIP-98 event must have:
- `kind`: 27235
- `u` tag: the exact request URL (e.g., `https://api.btcmap.org/v4/nostr/token`)
- `method` tag: `POST`
- `created_at`: within 60 seconds of server time
- Valid Schnorr signature

#### Example Request

```bash
curl -X POST https://api.btcmap.org/v4/nostr/token \
  -H "Authorization: Nostr <base64-encoded NIP-98 event>"
```

Note: The `Authorization` header uses the `Nostr` scheme (not `Bearer`), as specified by NIP-98.

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns a Bearer token |
| 400  | Bad Request - NIP-98 verification failed or no linked account |
| 401  | Unauthorized - Missing or invalid Authorization: Nostr header |

##### Example Response (200 OK)

```json
{
  "token": "550e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Description |
|-------|------|-------------|
| token | String | Bearer token (UUID v4) for authenticating subsequent API requests |

#### Usage Flow

```
1. User has a BTC Map account with a linked Nostr pubkey
2. User's Nostr client signs a NIP-98 event for POST https://api.btcmap.org/v4/nostr/token
3. Client sends: POST /v4/nostr/token with Authorization: Nostr <base64>
4. Server verifies event, looks up user by pubkey, returns Bearer token
5. Client uses Bearer token for all subsequent API calls
```
