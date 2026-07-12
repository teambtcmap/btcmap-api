# create_event

## Description

You can add Bitcoin-related events on BTC Map via this method. Most events are eiter conferences or community meetups.

## Params

```json
{
  "lat": 18.7822,
  "lon": 98.9942,
  "name": "Chiang Mai Weekly Meetup",
  "website": "https://www.meetup.com/bitcoinsinchiangmai/",
  "starts_at": "2025-08-28T19:00:00+07:00",
  "ends_at": null,
  "cron_schedule": null
}
```

We don't keep a lot of data about events due to our lack of maintaining capacity. That's why every eligeble event should have it's own website where users can look up all the details.

Most events are recurring and held at a fixed location. In these cases, `starts_at`, `ends_at`, and `cron_schedule` can all be omitted. The `website` link should direct users to a page with the up-to-date schedule, such as a dedicated event series website or a Meetup.com profile. The event will be displayed permanently (until it is deleted via `delete_event`), so hosts of fixed-location meetups only need to add the event once.

Example of a permanent event with no fixed schedule:

```json
{
  "lat": 18.7822,
  "lon": 98.9942,
  "name": "Chiang Mai Bitcoin Meetup",
  "website": "https://www.meetup.com/bitcoinsinchiangmai/",
  "starts_at": null,
  "ends_at": null,
  "cron_schedule": null
}
```

For a one-off event without a fixed end time, you may provide only the `starts_at` parameter.

The optional `cron_schedule` field accepts a cron expression that describes when the event recurs. It is used internally to refresh upcoming event instances and may be omitted for one-off events.

## Result Format

```json
{
  "id": 514
}
```

## Allowed Roles

- Root
- Admin
- Event Manager

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
  --data '{"jsonrpc":"2.0","method":"create_event","params":{"lat":18.7822,"lon":98.9942,"name":"Chiang Mai Weekly Meetup","website":"https://www.meetup.com/bitcoinsinchiangmai/","starts_at":"2025-08-28T19:00:00+07:00","ends_at":null,"cron_schedule":null},"id":1}' \
  https://api.btcmap.org/rpc
```
