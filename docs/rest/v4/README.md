# BTC Map REST API v4

The latest and recommended version of the BTCMap API, offering improved performance and features. 

## Endpoints  

### Implemented  
- **[Places](places.md)** - Fetch places. 
- **[Place Boosts](place-boosts.md)** - Fetch place boost quotes and submit boost intents. 
- **[Events](events.md)** - Fetch events.
- **[Invoices](invoices.md)** - Check invoice status for boosts, comments and other paywalled features.  
### Planned
- **[Areas](areas.md)** *(Coming soon)* 
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
| 404  | Not Found - Resource doesn't exist |
| 500  | Server Error - Unexpected failure in database or elsewhere |

### Error Response Body

```jsonc
{
  "code": "string",    // Machine-readable error identifier
  "message": "string"  // Human-readable explanation
}
```

