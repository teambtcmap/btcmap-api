# Dashboard REST API (v4)

This document describes the endpoint for fetching the internal
infrastructure / analytics dashboard in REST API v4.

## Available Endpoints

- [Get Infrastructure Dashboard](#get-infrastructure-dashboard)

### Get Infrastructure Dashboard

```bash
curl --request GET https://api.btcmap.org/v4/dashboard/infra \
  --header "Authorization: Bearer <token>"
```

Returns the same operational overview as the RPC `dashboard` method: place
create/update/delete counts, import-origin stats, request-log stats (including
the top RPC methods, REST calls and authenticated users over the last 24h),
unique-IP breakdown by platform, storage usage, LND node stats, recent sync
runs and wallet snapshots.

This endpoint is **private**. It requires a `Bearer` token whose effective
roles include `dashboard`, `admin` or `root`. Requests without a valid token
receive `401`, and authenticated users without a permitted role receive `403`.

#### Response Shape

```jsonc
{
  "started_at": "2026-10-10T11:22:39.375153298Z",
  "finished_at": "2026-10-10T11:22:40.112334459Z",
  "generation_time_ms": 737,
  "places": {
    "added":   { "d1": 8,  "d7": 98,  "d30": 347 },
    "updated": { "d1": 82, "d7": 239, "d30": 1173 },
    "deleted": { "d1": 9,  "d7": 24,  "d30": 118 }
  },
  "imports": [ /* per-origin totals / pending / revoked */ ],
  "logs": {
    "file_size_bytes": 0,
    "requests": { "d1": 0, "d7": 0, "d30": 0 },
    "top_rpcs": [ { "method": "get_element", "count": 0 } ],
    "top_rest_api_calls": [ { "method": "GET", "path": "/v4/places", "count": 0 } ],
    "top_users": [ { "user_id": 10, "name": "Scheduler", "count": 1819 } ]
  },
  "unique_ips_24h": { "web": 0, "android": 0, "ios": 0, "other_humans": 0, "bots": 0 },
  "storage": { "disks": [ /* per-disk usage */ ] },
  "lnd": null,
  "sync_runs": [ /* most recent sync runs */ ],
  "wallets": { "wallets": [ /* wallet snapshots */ ] }
}
```

`lnd` is `null` when no LND readonly macaroon is configured.
