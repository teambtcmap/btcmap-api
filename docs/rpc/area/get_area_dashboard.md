# get_area_dashboard

## Description

Returns the 365-day dashboard for an area: the latest totals from the most
recent [report](generate_reports.md) plus three daily time series used to
draw the area's charts on the BTC Map frontend.

The three charts are:

- `total_elements_chart` — total elements mapped to the area per day
- `verified_elements_365d_chart` — elements that were up-to-date as of that day
- `days_since_verified_chart` — average days since the elements were last verified

If no report exists for the area yet, the call fails with a server error.

## Params

```json
{
  "area_id": 123
}
```

| Field | Type | Description |
| --- | --- | --- |
| `area_id` | integer | Numeric area id |

## Result Format

```json
{
  "total_elements": 120,
  "verified_elements_365d": 87,
  "total_elements_chart": [
    { "date": "2024-09-09", "value": 118 },
    { "date": "2024-09-10", "value": 120 }
  ],
  "verified_elements_365d_chart": [
    { "date": "2024-09-09", "value": 85 },
    { "date": "2024-09-10", "value": 87 }
  ],
  "days_since_verified_chart": [
    { "date": "2024-09-09", "value": 42 },
    { "date": "2024-09-10", "value": 40 }
  ]
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli get-area-dashboard 123
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_area_dashboard","params":{"area_id":123},"id":1}' \
  https://api.btcmap.org/rpc
```
