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
  "ends_at": null
}
```

We don't keep a lot of data about events due to our lack of maintaining capacity. That's why every eligeble event should have it's own website where users can look up all the details.

Most events are recurring and held at a fixed location. `starts_at` is required: every event must have a start time. The `website` link should direct users to a page with the up-to-date schedule, such as a dedicated event series website or a Meetup.com profile. For recurring events, set `starts_at` to the next scheduled occurrence. `ends_at` is optional and may be omitted for an event without a fixed end time.

### Timestamps and timezones

`starts_at` and `ends_at` accept two forms:

- **With an explicit UTC offset** (RFC 3339), including `Z` for UTC:
  `"2025-08-28T19:00:00+07:00"` or `"2025-08-28T12:00:00Z"`. The offset is stored
  as given, so `Z` always means UTC. This is the original behaviour.
- **As a floating local time** with no offset: `"2025-08-28T19:00:00"`. Because a
  local time is ambiguous on its own, the `timezone` parameter is then required.

The optional `timezone` field accepts either an IANA zone name (e.g.
`Europe/Berlin`) or `"auto"`, which infers the zone from the event's `(lat, lon)`.
The server resolves the offset for that specific date, so daylight saving is
handled. Provide either explicit offsets or a `timezone`, not both.

The inferred or supplied zone is echoed back in the response so the caller can
verify what was applied.

The optional `area_id` field links the event to a [community area](../area/README.md). It is mainly relevant for [`event_manager`](../user-methods.md#set_user_geofence) callers, who must keep events inside their geofence either by linking an `area_id` in the fence or by placing `(lat, lon)` inside a fenced area.

## Result Format

```json
{
  "id": 514,
  "starts_at": "2025-08-28T19:00:00+07:00",
  "ends_at": null,
  "timezone": "Asia/Bangkok"
}
```

`timezone` is the zone applied to floating timestamps, or `null` when the
timestamps carried their own offsets.

## Allowed Roles

- Root
- Admin
- Event Manager

## Geofence Restriction

If the calling user has a non-empty
[geofence](../user-methods.md#set_user_geofence), the event must
either be linked to an `area_id` in the geofence, or its `(lat, lon)` must
fall inside the geometry of at least one area in the geofence. This restriction
applies equally to root, admin, and event manager callers. An empty geofence
means unrestricted.

## Examples

### btcmap-cli

```bash
btcmap-cli event create-event --name 'Chiang Mai Weekly Meetup' \
  --lat 18.7822 \
  --lon 98.9942 \
  --website 'https://www.meetup.com/bitcoinsinchiangmai/' \
  --starts-at '2025-08-28T19:00:00+07:00'
```

### curl

Explicit offset:

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_event","params":{"lat":18.7822,"lon":98.9942,"name":"Chiang Mai Weekly Meetup","website":"https://www.meetup.com/bitcoinsinchiangmai/","starts_at":"2025-08-28T19:00:00+07:00","ends_at":null},"id":1}' \
  https://api.btcmap.org/rpc
```

Floating local time with an inferred timezone:

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_event","params":{"lat":18.7822,"lon":98.9942,"name":"Chiang Mai Weekly Meetup","website":"https://www.meetup.com/bitcoinsinchiangmai/","starts_at":"2025-08-28T19:00:00","timezone":"auto"},"id":1}' \
  https://api.btcmap.org/rpc
```
