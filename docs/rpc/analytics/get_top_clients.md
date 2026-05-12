# get_top_clients

## Description

Returns a report of the top clients (IPs) grouped by platform (web, android, ios) over the last 24 hours. Results are sorted by unique_ips in descending order.

## Params

```json
{}
```

## Result Format

```json
[
  {
    "name": "android",
    "total_requests": 3000,
    "unique_ips": 200,
    "top_ips": [
      {
        "ip": "192.168.1.1",
        "count": 150
      }
    ]
  },
  {
    "name": "web",
    "total_requests": 8000,
    "unique_ips": 400,
    "top_ips": [
      {
        "ip": "10.0.0.1",
        "count": 500
      }
    ]
  },
  {
    "name": "ios",
    "total_requests": 1345,
    "unique_ips": 78,
    "top_ips": [
      {
        "ip": "172.16.0.1",
        "count": 100
      }
    ]
  }
]
```

## Allowed Roles

- Root
- Admin

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_top_clients","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```