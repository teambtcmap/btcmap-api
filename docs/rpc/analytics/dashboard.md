# dashboard

## Description

Returns a high-level analytics dashboard snapshot, including the time the report took to generate, counts of places added, updated, and deleted over the last 1, 7, and 30 days (from the `element_event` log), counts of imported places grouped by import origin over the same windows (from the `place_submission` table), log database stats (file size, number of logged requests, the 10 most-called RPC methods, and the 10 most-called REST API endpoints over the last 24 hours), the number of unique client IP addresses seen in the last 24 hours bucketed by platform (Web, Android, iOS, Other-humans, Bots) detected from the request's `User-Agent` header, disk usage stats for the host's real block devices, on-chain and Lightning channel balances probed from the LND node, the 10 most recent OSM sync runs recorded in the `sync` log table, and the BTC Map spending, donations, and treasury on-chain wallet balances derived from the xpubs configured in the `conf` table.

## Params

```json
{}
```

## Result Format

```json
{
  "started_at": "2024-12-31T23:59:00Z",
  "finished_at": "2024-12-31T23:59:00Z",
  "generation_time_ms": 12,
  "places": {
    "added": {
      "d1": 10,
      "d7": 50,
      "d30": 200
    },
    "updated": {
      "d1": 5,
      "d7": 30,
      "d30": 150
    },
    "deleted": {
      "d1": 1,
      "d7": 5,
      "d30": 20
    }
  },
  "imports": [
    {
      "origin": "square",
      "total": {
        "d1": 50,
        "d7": 300,
        "d30": 1200
      },
      "pending": {
        "d1": 10,
        "d7": 40,
        "d30": 80
      },
      "revoked": {
        "d1": 1,
        "d7": 4,
        "d30": 15
      }
    },
    {
      "origin": "coinos",
      "total": {
        "d1": 20,
        "d7": 120,
        "d30": 500
      },
      "pending": {
        "d1": 5,
        "d7": 25,
        "d30": 60
      },
      "revoked": {
        "d1": 0,
        "d7": 2,
        "d30": 6
      }
    }
  ],
  "logs": {
    "file_size_bytes": 2230968320,
    "requests": {
      "d1": 12000,
      "d7": 80000,
      "d30": 320000
    },
    "top_rpcs": [
      {
        "method": "revoke_submitted_place",
        "count": 9000
      },
      {
        "method": "get_area_dashboard",
        "count": 2000
      }
    ],
    "top_rest_api_calls": [
      {
        "method": "GET",
        "path": "/v2/elements",
        "count": 18000
      },
      {
        "method": "GET",
        "path": "/v4/places/search",
        "count": 16000
      },
      {
        "method": "POST",
        "path": "/v4/places",
        "count": 500
      }
    ]
  },
  "unique_ips_24h": {
    "web": 238,
    "android": 214,
    "ios": 89,
    "other_humans": 1668,
    "bots": 854
  },
  "storage": {
    "disks": [
      {
        "device": "/dev/mapper/root",
        "mount_point": "/",
        "total_bytes": 982277472256,
        "used_bytes": 475595489280,
        "available_bytes": 456709595136,
        "used_percent": 52.0
      },
      {
        "device": "/dev/nvme1n1p1",
        "mount_point": "/boot",
        "total_bytes": 1071628288,
        "used_bytes": 68329472,
        "available_bytes": 1003298816,
        "used_percent": 7.0
      }
    ]
  },
  "lnd": {
    "onchain_total_sat": 1500000,
    "onchain_confirmed_sat": 1500000,
    "onchain_unconfirmed_sat": 0,
    "outbound_liquidity_sat": 2750000,
    "inbound_liquidity_sat": 3200000,
    "pending_outbound_liquidity_sat": 0,
    "pending_inbound_liquidity_sat": 0,
    "total_balance_sat": 4250000
  },
  "sync_runs": [
    {
      "id": 42,
      "started_at": "2024-12-31T23:50:00Z",
      "finished_at": "2024-12-31T23:55:00Z",
      "duration_s": 300.5,
      "overpass_response_time_s": 12.3,
      "elements_affected": 120,
      "elements_created": 10,
      "elements_updated": 80,
      "elements_deleted": 30,
      "failed_at": null,
      "fail_reason": null
    }
  ],
  "wallets": {
    "spending": 125000,
    "donations": 84000,
    "treasury": 2100000,
    "fetched_at": "2024-12-31T23:55:00Z"
  }
}
```

## Fields

- `started_at`: UTC timestamp (RFC 3339) when the report generation started
- `finished_at`: UTC timestamp (RFC 3339) when the report generation finished
- `generation_time_ms`: Wall-clock time it took to generate the report, in milliseconds
- `places.added.d1` / `d7` / `d30`: Number of `create` events recorded in the last 1, 7, and 30 days
- `places.updated.d1` / `d7` / `d30`: Number of `update` events recorded in the last 1, 7, and 30 days
- `places.deleted.d1` / `d7` / `d30`: Number of `delete` events recorded in the last 1, 7, and 30 days
- `imports`: Imported places grouped by `origin` (e.g. `square`, `coinos`, `btcpayserver`), ordered alphabetically by `origin`. Only origins that had at least one submission created within the largest window (`d30`) are included. Each entry contains:
  - `origin`: Name of the import origin (matches `place_submission.origin`)
  - `total.d1` / `d7` / `d30`: Number of submissions created in the last 1, 7, and 30 days, regardless of state
  - `pending.d1` / `d7` / `d30`: Number of submissions created in the last 1, 7, and 30 days that are still open (`closed_at IS NULL AND revoked = 0`)
  - `revoked.d1` / `d7` / `d30`: Number of submissions created in the last 1, 7, and 30 days that have been explicitly revoked (`revoked = 1`)
- `logs.file_size_bytes`: Size of the `log.db` file on disk in bytes (0 if the file is missing)
- `logs.requests.d1` / `d7` / `d30`: Number of HTTP requests logged in the last 1, 7, and 30 days
- `logs.top_rpcs`: Up to 10 most-called RPC methods on the `/rpc` endpoint over the last 24 hours, ordered by `count` descending (most-called first). Each entry contains:
  - `method`: Name of the RPC method (e.g. `revoke_submitted_place`, `get_area_dashboard`)
  - `count`: Number of times the method was called in the window
- `logs.top_rest_api_calls`: Up to 10 most-called REST API endpoints (paths starting with `/v2`, `/v3`, `/v4`, or `/feeds`) over the last 24 hours, ordered by `count` descending and then by `path` and `method` ascending for deterministic ordering. `/rpc`, `/og/...`, static assets, and other non-REST paths are excluded. Each entry contains:
  - `method`: HTTP method of the request (e.g. `GET`, `POST`); may be empty for requests logged before the `method` column was added
  - `path`: Request path (e.g. `/v2/elements`, `/v4/places/search`)
  - `count`: Number of times this (method, path) combination was called in the window
- `unique_ips_24h`: Number of unique client IP addresses seen in the last 24 hours, bucketed by the platform inferred from the request's `User-Agent` header. Each IP is counted exactly once, assigned to the most specific bucket it matches. Order of precedence (first match wins): `bots`, `android`, `ios`, `web`, `other_humans`. Contains:
  - `web`: Distinct IPs whose `User-Agent` is exactly `btcmap.org` (the official web client)
  - `android`: Distinct IPs whose `User-Agent` starts with `BTC Map Android` (official Android client) or is exactly `okhttp/5.0.0-alpha.14` (an older build of the same client that hasn't been updated to set a custom `User-Agent`)
  - `ios`: Distinct IPs whose `User-Agent` contains `CFNetwork` (the official iOS client, which identifies as `BTCMap/<version> CFNetwork/...`)
  - `other_humans`: Distinct IPs whose requests don't match any of the four platform signatures or bot signatures — typically humans browsing btcmap.org in a regular browser (desktop or mobile Safari/Chrome), people using `curl` / `node` / scripts without a bot-like UA, or clients that send no `User-Agent` at all. This is the best proxy for "human traffic that isn't using one of the official apps"
  - `bots`: Distinct IPs whose requests match a known bot, crawler, link-preview, or test signature. Includes any UA containing `bot`, `spider`, or `crawler` (case-insensitive), plus `Zapier`, `Twitterbot`, `facebookexternalhit`, `meta-externalagent`, `Applebot`, `AhrefsBot`, `SemrushBot`, `DuckDuckBot`, `Bytespider`, and `btcmap-e2e-tests`. Search-engine crawlers (Googlebot, Bingbot, Baiduspider, Sogou, DuckDuckBot, Amazonbot, etc.) fall in this bucket. An IP that sends a mix of bot and non-bot requests in the window is classified as a bot (bots take precedence)
- `storage.disks`: Disk usage stats for the host's real block devices (e.g. `/dev/sda1`, `/dev/mapper/root`, `/dev/nvme0n1p1`). Virtual filesystems such as `tmpfs`, `devtmpfs`, `sysfs`, `proc`, `overlay`, and `efivarfs` are excluded. Sourced from `df -PB1`. Each entry contains:
  - `device`: Device file path (always starts with `/dev/`)
  - `mount_point`: Where the device is mounted
  - `total_bytes`: Total size of the device in bytes
  - `used_bytes`: Bytes currently in use
  - `available_bytes`: Bytes currently free
  - `used_percent`: Percentage of the device that is in use (0.0–100.0)
- `lnd`: Balances probed from the LND Lightning node (`https://lnd.btcmap.org`) using the `lnd_readonly_macaroon` from the `conf` table (requires at least `info:read`, `onchain:read`, and `offchain:read` permissions). `null` if the macaroon is unset or the node is unreachable. When present, contains:
  - `onchain_total_sat`: Total on-chain wallet balance in satoshis (confirmed + unconfirmed)
  - `onchain_confirmed_sat`: Confirmed on-chain wallet balance in satoshis
  - `onchain_unconfirmed_sat`: Unconfirmed on-chain wallet balance in satoshis
  - `outbound_liquidity_sat`: Total local channel balance in satoshis (sendable via Lightning)
  - `inbound_liquidity_sat`: Total remote channel balance in satoshis (receivable via Lightning)
  - `pending_outbound_liquidity_sat`: Local balance locked in channels that are still being opened, in satoshis
  - `pending_inbound_liquidity_sat`: Remote balance in channels that are still being opened, in satoshis
  - `total_balance_sat`: Total funds the node controls in satoshis (`onchain_confirmed_sat` + `outbound_liquidity_sat` + `pending_outbound_liquidity_sat`)
- `sync_runs`: Up to 10 most recent OSM sync runs, ordered by `started_at` descending (most recent first). Each entry contains:
  - `id`: Sync run ID
  - `started_at`: UTC timestamp (RFC 3339) when the sync started
  - `finished_at`: UTC timestamp (RFC 3339) when the sync finished, or `null` if it is still running or failed
  - `duration_s`: Wall-clock duration of the sync in seconds, or `null` if it did not finish
  - `overpass_response_time_s`: Time the Overpass API call took in seconds, or `null` if it did not finish
  - `elements_affected`: Total number of elements created, updated, or deleted by the run
  - `elements_created`: Number of new elements added by the run
  - `elements_updated`: Number of existing elements updated by the run
  - `elements_deleted`: Number of elements marked as deleted by the run
  - `failed_at`: UTC timestamp (RFC 3339) when the sync failed, or `null` on success
  - `fail_reason`: Human-readable failure reason, or `null` on success
- `wallets`: BTC Map on-chain wallet balances in satoshis, derived by summing the script getBalance response from Electrum for every script derived from each configured xpub (gap limit 20). Balances are served from an in-memory cache that is warmed once at server startup and refreshed every 5 minutes by a background task, so values may be up to ~5 minutes stale. When no xpubs are configured all three balances are `0`. If the Electrum probe fails and there is no cached snapshot, all three balances fall back to `0` and `fetched_at` is `null`. Contains:
  - `spending`: Balance of the spending wallet, derived from `conf.xpub_spending`
  - `donations`: Balance of the donations wallet, derived from `conf.xpub_donations`
  - `treasury`: Balance of the treasury wallet, derived from `conf.xpub_treasury`
  - `fetched_at`: UTC timestamp (RFC 3339) when these balances were last probed against Electrum, or `null` if the probe failed and no cached snapshot exists

Windows are calculated from "now" at the time of the request. The `d1` window is included in `d7`, and `d7` is included in `d30`.

## Allowed Roles

- Root
- Admin

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"dashboard","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
