# Incremental Sync (Recommended for Native Apps)

For performance-critical apps, use BTC Map's incremental sync API with native map widgets. Avoid slower web views. 

## Reduce Latency  

The primary API server (`api.btcmap.org`) is hosted in London. To minimize latency:  
- Use the **CDN-backed static snapshot** (below) for initial data.  
- Fetch incremental updates from the API only after. 

## CDN-Powered Static Snapshots  

Merchant data changes slowly (~few %/month). Fetch the latest snapshot from:

https://cdn.static.btcmap.org/api/v4/places.json

This file is usually already cached by a few CDN nodes close to your physical location, so it should allow you to fetch all BTC Map merchants in well under a second, in most cases.

**Included Fields:**
- `id` – Use to look up more fields on demand
- `lat`, `lon` – Location data  
- `icon`, `comments` – UI metadata
- `boosted_until` – Boost expiry date, returned for currently boosted places only

Which should be enough to display all the pins on your map, with proper icons and comment count badges.

## Incremental Updates 

1. Note the `last-modified` header (e.g., `2025-06-11T00:00:00Z`).  
2. Fetch changes since that timestamp:

```bash
curl `https://api.btcmap.org/v4/places?fields=id,lat,lon,icon,comments,deleted_at
  &updated_since=2025-06-11T00:00:00Z
  &include_deleted=true`
```

Key Parameters:

- `updated_since`: Sync anchor timestamp.
- `include_deleted`: Required to evict stale records.

## Example Sync Flow

```bash
// 1. Initial CDN Snapshot
curl `https://cdn.static.btcmap.org/api/v4/places.json`

// 2. First Incremental Update
curl `https://api.btcmap.org/v4/places?fields=id,lat,lon,icon,comments,deleted_at
  &updated_since=2025-06-11T00:00:00Z
  &include_deleted=true`
```
