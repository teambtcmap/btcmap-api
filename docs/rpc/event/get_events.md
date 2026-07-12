# get_events

## Description

Retrieves events. By default, soft-deleted and past events are excluded.

## Params

```json
{
  "include_past": false,
  "include_deleted": false
}
```

| Field             | Type    | Default | Description                                                                                     |
| ----------------- | ------- | ------- | ----------------------------------------------------------------------------------------------- |
| `include_past`    | boolean | `false` | When `true`, events whose `starts_at` is in the past are included.                              |
| `include_deleted` | boolean | `false` | When `true`, soft-deleted events (those with a non-null `deleted_at`) are included.             |

Events with no `starts_at` (permanent events) are always included regardless of `include_past`, since they have no notion of being past.

## Result Format

```json
[
  {
    "id": 1,
    "lat": 7.8812324,
    "lon": 98.3884695,
    "name": "Phuket Bitcoin Meetup",
    "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
    "starts_at": "2025-08-29T19:00:00+07:00",
    "ends_at": null,
    "cron_schedule": null
  }
]
```

## Allowed Roles

- Root
- Admin
- Event Manager

## Examples

### btcmap-cli

```bash
btcmap-cli get-events
btcmap-cli get-events --include-past
btcmap-cli get-events --include-deleted
btcmap-cli get-events --include-past --include-deleted
```

### curl

Default (only upcoming and permanent events):

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_events","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```

Include past events:

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_events","params":{"include_past":true},"id":1}' \
  https://api.btcmap.org/rpc
```

Include soft-deleted events:

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_events","params":{"include_deleted":true},"id":1}' \
  https://api.btcmap.org/rpc
```
