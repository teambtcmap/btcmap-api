# generate_reports

## Description

Builds today's daily report for every area, capturing metrics like total
elements, payment-method breakdowns (onchain, lightning, lightning
contactless, legacy), counts of merchants and ATMs, and the percentage of
up-to-date elements. Each report is stored in the `report` table and is what
the [get_area_dashboard](get_area_dashboard.md) endpoint reads from.

If a report for today already exists, the call is a no-op and returns
`new_reports: 0`. The response includes the wall-clock duration of the run.

## Params

Empty params object:

```json
{}
```

## Result Format

```json
{
  "started_at": "2024-09-10T00:00:00Z",
  "finished_at": "2024-09-10T00:00:15Z",
  "time_s": 14.8,
  "new_reports": 15
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli generate-reports
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"generate_reports","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
