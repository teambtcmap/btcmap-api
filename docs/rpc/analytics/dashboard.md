# dashboard

## Description

Returns a high-level analytics dashboard snapshot, including the time the report took to generate, counts of places added, updated, and deleted over the last 1, 7, and 30 days (from the `element_event` log), log database stats (file size, number of logged requests, and the 10 most-called RPC methods over the same windows), and the 10 most recent OSM sync runs recorded in the `sync` log table.

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
    ]
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
  ]
}
```

## Fields

- `started_at`: UTC timestamp (RFC 3339) when the report generation started
- `finished_at`: UTC timestamp (RFC 3339) when the report generation finished
- `generation_time_ms`: Wall-clock time it took to generate the report, in milliseconds
- `places.added.d1` / `d7` / `d30`: Number of `create` events recorded in the last 1, 7, and 30 days
- `places.updated.d1` / `d7` / `d30`: Number of `update` events recorded in the last 1, 7, and 30 days
- `places.deleted.d1` / `d7` / `d30`: Number of `delete` events recorded in the last 1, 7, and 30 days
- `logs.file_size_bytes`: Size of the `log.db` file on disk in bytes (0 if the file is missing)
- `logs.requests.d1` / `d7` / `d30`: Number of HTTP requests logged in the last 1, 7, and 30 days
- `logs.top_rpcs`: Up to 10 most-called RPC methods on the `/rpc` endpoint over the last 24 hours, ordered by `count` descending (most-called first). Each entry contains:
  - `method`: Name of the RPC method (e.g. `revoke_submitted_place`, `get_area_dashboard`)
  - `count`: Number of times the method was called in the window
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
