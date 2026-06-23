# Auth REST API (v4)

This document describes the authentication endpoints in REST API v4.

BTC Map issues opaque Bearer tokens. A token can be obtained in two ways:

- **Username + password** — see [Create Token](users.md#create-token).
- **Nostr (NIP-98)** — sign in with a Nostr keypair, described below.

Both paths mint the same kind of opaque Bearer token, sent as
`Authorization: Bearer <token>` on subsequent requests.

> [!NOTE]
> The `/v4/auth` scope is **Nostr-specific** — it currently exposes only the
> endpoint below. Password login is **not** under `/v4/auth`; it lives at
> `POST /v4/users/{username}/tokens` ([Create Token](users.md#create-token)),
> which takes the password in an `Authorization: Bearer <password>` header.

## Available Endpoints

- [Sign In with Nostr](#sign-in-with-nostr)

## Server Configuration (NIP-98)

> [!IMPORTANT]
> Every NIP-98 endpoint (this one and [Link Nostr Identity](users.md#link-nostr-identity))
> verifies the signed event's `u` tag against the API's **configured public
> base URL**, taken from the `BTCMAP_API_BASE_URL` environment variable — **not**
> from the request's `Host`/`X-Forwarded-*` headers (trusting those would let an
> attacker replay an event signed for another host).

`BTCMAP_API_BASE_URL` **must be set to the public origin clients actually call**,
e.g.:

```
BTCMAP_API_BASE_URL=https://api.btcmap.org
```

It defaults to `http://127.0.0.1:8000` (the local dev address). If the deployed
value does not match the origin the client signs against, the `u` tags will
never match and **all NIP-98 requests fail with `401`** — even though the routes
themselves are reachable. This failure is silent: there is no startup warning,
and non-Nostr endpoints are unaffected, so a misconfigured deployment looks
healthy until someone tries to sign in with Nostr.

### Sign In with Nostr

Exchanges a NIP-98 signed event for a Bearer token. If no account is linked to
the signing pubkey yet, one is **auto-created** (random username, no password,
`user` role, npub set) and a token is returned for it. Subsequent sign-ins with
the same key return a token for that same account.

The signed event is carried on the `Authorization` header with the `Nostr`
scheme. The event must be a NIP-98 event (kind `27235`) that signs:

- `u` = `<BTCMAP_API_BASE_URL>/v4/auth/nostr`
- `method` = `POST`

within a ±60s window of the server clock. Both `u` and `method` are matched
case-sensitively. See [Server Configuration](#server-configuration-nip-98) above
for what `<BTCMAP_API_BASE_URL>` resolves to.

#### Example Request

```bash
curl -X POST https://api.btcmap.org/v4/auth/nostr \
  -H "Authorization: Nostr <base64-encoded-nip98-event>"
```

#### Response

| Code | Description |
|------|-------------|
| 200  | Success - Returns a Bearer token and the signing identity |
| 401  | Unauthorized - Missing/invalid NIP-98 proof, or `u`/`method`/signature/timestamp mismatch |

##### Example Response (200 OK)

```json
{
  "token": "df6cfea3-6169-40ed-945f-f8e5690a1ce7",
  "username": "exuberant-street-7342",
  "npub": "npub1..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| token | String | Bearer token to send as `Authorization: Bearer <token>` on subsequent requests |
| username | String | Username of the signed-in (or newly created) account |
| npub  | String | Bech32 npub of the Nostr identity that signed in |
