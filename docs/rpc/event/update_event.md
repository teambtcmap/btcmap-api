# update_event

## Description

Partially updates an existing event. Only the fields you send are changed; omitted fields are left untouched. This means a single-field rename, a coordinate fix, or a schedule tweak all use the same call.

For the nullable fields (`area_id`, `starts_at`, `ends_at`), pass an explicit `null` to clear the value. Omitting the field keeps whatever is currently stored. For the non-nullable fields (`lat`, `lon`, `name`, `website`), sending `null` is also treated as "leave unchanged" — pass an actual value to overwrite.

The event's `id` cannot be changed via this endpoint. The response returns the event as it looks after the update. If no field is supplied (only `id`), the call is a no-op and returns the existing row without bumping `updated_at`.

## Params

```json
{
  "id": 1,
  "name": "Phuket Bitcoin Meetup (renamed)"
}
```

| Field       | Type           | Description                                                                                     |
| ----------- | -------------- | ----------------------------------------------------------------------------------------------- |
| `id`        | integer        | Required. ID of the event to update.                                                              |
| `area_id`   | integer \| null | Optional. Pass an area id to link, `null` to unlink. Omit to leave unchanged.                    |
| `lat`       | number         | Optional. Latitude in decimal degrees. Omit to leave unchanged.                                   |
| `lon`       | number         | Optional. Longitude in decimal degrees. Omit to leave unchanged.                                  |
| `name`      | string         | Optional. Display name of the event. Omit to leave unchanged.                                    |
| `website`   | string         | Optional. URL with up-to-date event details. Omit to leave unchanged.                            |
| `starts_at` | string \| null | Optional. Start time. Pass `null` to clear (permanent event with no fixed schedule).             |
| `ends_at`   | string \| null | Optional. End time. Pass `null` to clear.                                                        |
| `timezone`  | string         | Optional. `"auto"` or an IANA zone name. Required when a timestamp you send has no UTC offset.    |

### Timestamps and timezones

`starts_at` and `ends_at` accept either an RFC 3339 timestamp with an explicit
offset (including `Z` for UTC, stored as given) or a floating local time with no
offset, e.g. `"2025-08-29T19:00:00"`. Floating values require `timezone` in the
same request (`"auto"` infers it from the event's `(lat, lon)`; any IANA name
selects that zone), and the offset is resolved for that date, daylight saving
included. The zone that was applied is echoed back in the response.

## Result Format

The updated event, identical in shape to `get_event`, plus the `timezone` that
was applied in this request:

```json
{
  "id": 1,
  "lat": 7.8812324,
  "lon": 98.3884695,
  "name": "Phuket Bitcoin Meetup (renamed)",
  "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
  "starts_at": "2025-08-29T19:00:00+07:00",
  "ends_at": null,
  "area_id": null,
  "timezone": null
}
```

If no event with the given `id` exists, the call fails with a server error.

## Allowed Roles

- Root
- Admin
- Event Manager

## Geofence Restriction

If the calling user has a non-empty
[geofence](../user-methods.md#set_user_geofence), the event state after the
update must satisfy the geofence — either its resulting `area_id` is in the
geofence, or its resulting `(lat, lon)` falls inside the geometry of at least
one area in the geofence. This restriction applies equally to root, admin,
and event manager callers. An empty geofence means unrestricted.

## Examples

### btcmap-cli

```bash
btcmap-cli event update-event 1 --name 'Phuket Bitcoin Meetup (renamed)'
```

To clear a nullable field instead of setting it, use the matching `--clear-*` flag:

```bash
# Unlink the event from its area and drop its fixed schedule
btcmap-cli event update-event 1 --clear-area-id --clear-starts-at --clear-ends-at
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"update_event","params":{"id":1,"name":"Phuket Bitcoin Meetup (renamed)"},"id":1}' \
  https://api.btcmap.org/rpc
```
