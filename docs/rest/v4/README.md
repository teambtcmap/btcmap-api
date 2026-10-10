# BTC Map REST API v4

The latest and recommended version of the BTCMap API, offering improved performance and features. 

## Endpoints  

### Implemented  
- **[Auth](auth.md)** - Sign in with Nostr (NIP-98) to obtain a Bearer token, and sign out to revoke it.
- **[Places](places.md)** - Fetch places.
- **[Place Boosts](place-boosts.md)** - Fetch place boost quotes and submit boost intents.
- **[Place Comments](place-comments.md)** - Fetch place comment quotes and submit comment intents.
- **[Place Images](place-images.md)** - Fetch photos attached to a place, upload new place photos as a signed-in user, list your own uploads, and delete them; admins and roots can list the most recent uploads across all places.
- **[Place Issues](place-issues.md)** - Fetch issues for places within an area.
- **[Place Reports](place-reports.md)** - File a report against an existing place as the `user` origin.
- **[Place Submissions](place-submissions.md)** - Fetch open, non-revoked place submissions (from external import sources like Square or CoinOS, or signed-in users) and submit new places as the `user` origin.
- **[Notes](notes.md)** - Drop free-form notes at a coordinate as a signed-in user, keep them private or public, and search public notes around a point.
- **[Events](events.md)** - Fetch events, submit new ones as a signed-in user, revoke your own pending submissions, track the status of your submissions, and review them as an event manager, admin or root.
- **[Activity](activity.md)** - Fetch a merged feed of place activity, optionally scoped to areas and/or places.
- **[Invoices](invoices.md)** - Check invoice status for boosts, comments and other paywalled features.
- **[Users](users.md)** - Get authenticated user information, and, as an admin or root, search users and update their roles and geofence.
- **[Areas](areas.md)** - Fetch areas, create and update areas as an area manager, admin or root, manage saved areas, and fetch per-area image and daily report data.
- **[Search](search.md)** - Search areas and places by name, address and any other OSM tag.
### Proposed
- Need something extra? Let us know!

## Incremental Sync  
For performance-sensitive apps with persistent caching:  
- [Sync Guide](sync.md) – Maintain a local data snapshot for instant (offline) retrieval. 

## Error Response Format

All API errors return:
1. Standard HTTP status codes
2. Consistent JSON error bodies

### HTTP Status Codes

| Code | Description |
|------|-------------|
| 400  | Bad Request - Invalid parameters |
| 401  | Unauthorized - Missing or invalid credentials |
| 403  | Forbidden - Authenticated but not allowed to perform the action |
| 404  | Not Found - Resource doesn't exist |
| 500  | Server Error - Unexpected failure in database or elsewhere |

### Error Response Body

```jsonc
{
  "code": "string",    // Machine-readable error identifier
  "message": "string"  // Human-readable explanation
}
```

