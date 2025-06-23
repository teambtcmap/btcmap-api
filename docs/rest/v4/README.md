# BTC Map REST API v4

The latest and recommended version of the BTCMap API, offering improved performance and features. 

## Endpoints  

### Implemented  
- **[Places](places.md)** – Fetch places (single or batch).  
### Planned  
- **[Areas](areas.md)** *(Coming soon)* 
### Proposed
- Need something extra? Let us know!

## Incremental Sync  
For performance-sensitive apps with persistent caching:  
- [Sync Guide](sync.md) – Maintain a local data snapshot for instant (offline) retrieval. 

## Error Handling  
**Current (Temporary):**  
- All errors return HTTP `500` with a plain-text message.  
- Treat any `500` response as an API error; the body may be displayed directly.  

**Planned Improvement:**  
- Adopt [RFC 9457](https://datatracker.ietf.org/doc/html/rfc9457) (Problem Details for HTTP APIs) for structured errors.  
- Proper status codes (e.g., `400` for client errors, `404` for not found).  

