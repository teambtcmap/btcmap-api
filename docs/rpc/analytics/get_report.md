# get_report

## Description

Returns an analytics report comparing place statistics between two dates. Includes total places, verified places (up to date within 1 year), average days since verification, boosts, and comments within the specified date range.

## Params

```json
{
  "start": "2024-01-01T00:00:00Z",
  "end": "2024-12-31T23:59:59Z"
}
```

## Result Format

```json
{
  "total_places_start": 5000,
  "total_places_end": 5500,
  "total_places_change": 500,
  "verified_places_1y_start": 4500,
  "verified_places_1y_end": 5000,
  "verified_places_1y_change": 500,
  "days_since_verified_start": 120,
  "days_since_verified_end": 130,
  "days_since_verified_change": 10,
  "boosts": 25,
  "boosts_total_days": 365,
  "comments": 150
}
```

## Fields

- `total_places_start`: Total places at the start date
- `total_places_end`: Total places at the end date
- `total_places_change`: Net change in total places
- `verified_places_1y_start`: Places verified within 1 year at start date
- `verified_places_1y_end`: Places verified within 1 year at end date
- `verified_places_1y_change`: Net change in verified places
- `days_since_verified_start`: Average days since verification at start date
- `days_since_verified_end`: Average days since verification at end date
- `days_since_verified_change`: Net change in average days since verification
- `boosts`: Number of paid boosts created within the date range
- `boosts_total_days`: Sum of boost duration days within the date range
- `comments`: Number of non-deleted comments created within the date range

## Allowed Roles

- Root

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_report","params":{"start":"2024-01-01T00:00:00Z","end":"2024-12-31T23:59:59Z"},"id":1}' \
  https://api.btcmap.org/rpc
```