# delete_event

## Description

Everything flows, everything changes. If an event's location has changed or it's been cancelled, use this method to remove it from BTC Map.

## Params

```json
{
  "id": 1
}
```

## Result Format

```json
{
  "id": 1
}
```

## Allowed Roles

- Root
- Admin
- Event Manager

## Geofence Restriction

If the calling user holds the `event_manager` role and has a non-empty
[geofence](../user-methods.md#set_user_geofence), the existing event being
deleted must already satisfy the geofence — either its `area_id` is in the
geofence, or its stored `(lat, lon)` falls inside the geometry of at least
one area in the geofence. Admins and roots are not subject to this check.
An empty geofence means unrestricted.

## Examples

### btcmap-cli

```bash
btcmap-cli delete-event 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"delete_event","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```
