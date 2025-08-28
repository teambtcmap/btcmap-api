# create_event

## Description

You can add Bitcoin-related events on BTC Map via this method. Most events are eiter conferences or community meetups.

## Params

```json
{
  "lat": 18.7822,
  "lon": 98.9942,
  "name": "Chiang Mai Weekly Meetup"
  "website": "https://www.meetup.com/bitcoinsinchiangmai/"
  "starts_at": "2025-08-28T19:00:00+07:00",
  "ends_at": null
}
```

We don't keep a lot of data about events due to our lack of maintaining capacity. That's why every eligeble event should have it's own website where users can look up all the details.

Most meetups start in the evening and have no fixed end time, so `ends_at` is optional in that case.

## Result Format

```json
{
  "id": 514
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

```bash
btcmap-cli create-event --name 'Chiang Mai Weekly Meetup' \
  --lat 18.7822 \
  --lon 98.9942 \
  --website 'https://www.meetup.com/bitcoinsinchiangmai/' \
  --starts-at '2025-08-28T19:00:00+07:00'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_event","params":{"lat":18.7822,"lon":98.9942,"name":"Chiang Mai Weekly Meetup","website":"https://www.meetup.com/bitcoinsinchiangmai/","starts_at":"2025-08-28T19:00:00+07:00","ends_at":null},"id":1}' \
  https://api.btcmap.org/rpc
```
