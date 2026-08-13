# get_trending_communities

## Description

Returns communities (areas whose `type` tag is `community`) that trended
during the given window. An area trends when elements inside it gain a
notable amount of activity — new places, updates, comments, or events —
between `period_start` and `period_end`. Both bounds are inclusive
day-resolution dates.

## Params

```json
{
  "period_start": "2024-09-01",
  "period_end": "2024-09-10"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `period_start` | string | Start date (inclusive), formatted `YYYY-MM-DD` |
| `period_end` | string | End date (inclusive), formatted `YYYY-MM-DD` |

## Result Format

```json
[
  {
    "id": 456,
    "name": "Austin",
    "url": "https://btcmap.org/area/austin",
    "events": 4,
    "created": 1,
    "updated": 3,
    "deleted": 0,
    "comments": 22
  }
]
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli get-trending-communities 2024-09-01 2024-09-10
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_trending_communities","params":{"period_start":"2024-09-01","period_end":"2024-09-10"},"id":1}' \
  https://api.btcmap.org/rpc
```
