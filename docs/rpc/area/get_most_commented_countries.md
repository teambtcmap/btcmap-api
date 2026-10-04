# get_most_commented_countries

## Description

Returns countries (areas whose `type` tag is `country`) ranked by the number
of comments left on their elements during the given window. Both bounds are
inclusive day-resolution dates. The response shape matches
[get_trending_countries](get_trending_countries.md); the ranking comes purely
from the `comments` field.

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
    "id": 49,
    "name": "United States",
    "url": "https://btcmap.org/area/us",
    "events": 12,
    "created": 0,
    "updated": 12,
    "deleted": 0,
    "comments": 87
  }
]
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli get-most-commented-countries 2024-09-01 2024-09-10
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_most_commented_countries","params":{"period_start":"2024-09-01","period_end":"2024-09-10"},"id":1}' \
  https://api.btcmap.org/rpc
```
