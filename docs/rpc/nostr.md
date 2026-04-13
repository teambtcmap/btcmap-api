# Nostr RPCs

These methods allow linking a Nostr identity to a BTC Map account and authenticating via [NIP-98](https://github.com/nostr-protocol/nips/blob/master/98.md) HTTP Auth.

## Table of Contents

- [link_nostr_identity](#link_nostr_identity)
- [update_nostr_identity](#update_nostr_identity)
- [remove_nostr_identity](#remove_nostr_identity)
- [create_api_key_with_nostr](#create_api_key_with_nostr)

## NIP-98 Event Requirements

For methods that accept a NIP-98 event, the base64-encoded Nostr event must satisfy:

- `kind`: 27235
- `u` tag: must exactly match the request URL (e.g., `https://api.btcmap.org/rpc`)
- `method` tag: `POST`
- `created_at`: within 60 seconds of server time
- Valid Schnorr signature

## link_nostr_identity

Links a Nostr pubkey to the authenticated user's BTC Map account. Each pubkey can only be linked to one account, and each account can only have one linked pubkey.

**Required Role**: User

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "link_nostr_identity",
  "params": {
    "nostr_event": "<base64-encoded NIP-98 kind 27235 event>",
    "url": "https://api.btcmap.org/rpc"
  },
  "id": 1
}
```

| Param | Type | Description |
|-------|------|-------------|
| nostr_event | String | Base64-encoded NIP-98 event signed by the Nostr key to link |
| url | String | The URL the NIP-98 event was signed for (must match the `u` tag) |

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "npub": "abc123def456..."
  },
  "id": 1
}
```

### Errors

- User already has a linked Nostr identity (use `update_nostr_identity` instead)
- Nostr pubkey is already linked to another account
- NIP-98 event verification failed

### Example

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"link_nostr_identity","params":{"nostr_event":"<base64>","url":"https://api.btcmap.org/rpc"},"id":1}' \
  https://api.btcmap.org/rpc
```

## update_nostr_identity

Replaces the Nostr pubkey linked to the authenticated user's account with a new one. The NIP-98 event must be signed by the **new** Nostr key.

**Required Role**: User

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "update_nostr_identity",
  "params": {
    "nostr_event": "<base64-encoded NIP-98 event signed by NEW key>",
    "url": "https://api.btcmap.org/rpc"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "npub": "newpubkey789..."
  },
  "id": 1
}
```

### Errors

- No Nostr identity linked (use `link_nostr_identity` first)
- New pubkey is already linked to another account
- NIP-98 event verification failed

### Example

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_nostr_identity","params":{"nostr_event":"<base64>","url":"https://api.btcmap.org/rpc"},"id":1}' \
  https://api.btcmap.org/rpc
```

## remove_nostr_identity

Removes the Nostr identity linked to the authenticated user's account.

**Required Role**: User

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_nostr_identity",
  "id": 1
}
```

No params required.

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "message": "Nostr identity removed"
  },
  "id": 1
}
```

### Errors

- No Nostr identity linked to this account

### Example

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"remove_nostr_identity","id":1}' \
  https://api.btcmap.org/rpc
```

## create_api_key_with_nostr

Obtain a BTC Map API Bearer token using NIP-98 Nostr authentication. This is the "login with Nostr" flow -- no password or existing Bearer token is required.

**Authentication**: `Authorization: Nostr <base64-encoded NIP-98 event>` header (not Bearer)

**Required Role**: None (anonymous method)

**Prerequisites**: The Nostr pubkey must already be linked to a BTC Map account via `link_nostr_identity`.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "create_api_key_with_nostr",
  "id": 1
}
```

No params required. Authentication is via the `Authorization: Nostr` header.

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "token": "550e8400-e29b-41d4-a716-446655440000",
    "time_ms": 5
  },
  "id": 1
}
```

| Field | Type | Description |
|-------|------|-------------|
| token | String | Bearer token (UUID v4) for subsequent API calls |
| time_ms | Number | Processing time in milliseconds |

### Errors

- Missing or invalid `Authorization: Nostr` header
- NIP-98 event verification failed
- No BTC Map account linked to this Nostr pubkey

### Example

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Nostr <base64-encoded-nip98-event>" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_api_key_with_nostr","id":1}' \
  https://api.btcmap.org/rpc
```

### Usage Flow

```
1. User already has a BTC Map account with linked Nostr pubkey
2. Nostr client signs a NIP-98 event for POST https://api.btcmap.org/rpc
3. Send RPC request with Authorization: Nostr <base64>
4. Server verifies event -> looks up user by pubkey -> returns Bearer token
5. Use Bearer token for all subsequent API/RPC calls
```
