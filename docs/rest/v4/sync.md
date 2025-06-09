# Incremental Sync Approach (Recommended for Native Apps)

The `/v4/places` endpoint is designed for efficient incremental synchronization. Clients should:

1. Store the timestamp of their last sync locally
2. Request elements that have been updated since that timestamp using the `updated_since` parameter
3. Deleted places can be excluded during the first sync but you need to include deleted places for follow-up sync in order to invalidate previously cached but now deleted entries
4. Process only the changes since the last sync
5. Use `max(updated_at)` as a starting point for the follow-up sync jobs

This approach minimizes data transfer and processing requirements, making it ideal for mobile applications and other bandwidth-constrained environments.

## Example Incremental Sync Flow

```
// Initial sync - store returned timestamp
GET /v4/places?updated_since=2020-01-01T00:00:00Z&limit=1000

// Subsequent sync - use timestamp from previous response
GET /v4/places?updated_since=2023-09-15T14:30:45Z&limit=1000
```