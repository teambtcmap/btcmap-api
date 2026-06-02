# dashboard

## Description

Returns a high-level analytics dashboard snapshot, including the time the report took to generate and counts of places added, updated, and deleted over the last 1, 7, and 30 days. Counts are derived from the `element_event` log (events with type `create`, `update`, and `delete`).

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
