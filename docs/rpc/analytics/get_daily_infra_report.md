# get_daily_infra_report

## Description

Returns a daily infrastructure report containing request statistics, unique IP counts, platform breakdowns (web, android, ios), top user agents, and user statistics (total users, new users in last 1 day, new users in last 1 month).

## Params

```json
{}
```

## Result Format

```json
{
  "total_requests": 12345,
  "unique_ips": 678,
  "web": {
    "requests": 8000,
    "unique_ips": 400
  },
  "android": {
    "requests": 3000,
    "unique_ips": 200
  },
  "ios": {
    "requests": 1345,
    "unique_ips": 78
  },
  "top_user_agents": [
    {
      "user_agent": "Mozilla/5.0 ...",
      "count": 500,
      "unique_ips": 150
    }
  ],
  "user_stats": {
    "total": 5000,
    "new_1d": 10,
    "new_1m": 150
  }
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli get-daily-infra-report
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_daily_infra_report","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```