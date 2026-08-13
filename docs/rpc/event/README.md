# Event RPC

Methods for managing events distributed via BTC Map REST API. Most events are
either conferences or recurring community meetups, pinned to a fixed location
via `(lat, lon)` and optionally linked to a particular [community](../area/README.md).

Each event has its own row in the `event` table. Deletion is soft (`deleted_at`
is set to non-null deletion timestamp). Use `get_events` with `include_deleted` to surface tombstones, and
`include_past` to surface events whose `starts_at` is already in the past.
Events without a `starts_at` are treated as permanent and are always returned
by `get_events`.

If the calling user has a non-empty
[geofence](../user-methods.md#set_user_geofence), `create_event`,
`update_event`, and `delete_event` enforce that the event falls inside the
geofence. This applies equally to
root, admin, and event manager callers.

- [create_event](create_event.md) - Add a new event
- [get_event](get_event.md) - Retrieve a single event by id
- [get_events](get_events.md) - List events, with optional past/deleted filtering
- [update_event](update_event.md) - Partially update an existing event
- [delete_event](delete_event.md) - Soft-delete an event
