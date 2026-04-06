# Area Feed API (v4)

This document describes the area feed endpoint which returns a combined chronological feed of edits, comments, and boosts for a given area.

## Available Endpoints

- [Get Area Feed](#get-area-feed)

### Get Area Feed

```
GET /v4/areas/{area_id_or_alias}/feed
```

Retrieves a combined activity feed for a specific area. The feed merges events (edits), comments, and boosts for all places within the area into a single chronological list, sorted by most recent first.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `area_id_or_alias` | Integer or String | **Required**. The area ID or URL alias (e.g., `thailand` or `42`). |

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | Integer | Optional. Maximum number of items to return. Default: `50`, max: `100`. |
| `after` | ISO 8601 datetime | Optional. Cursor for pagination — returns items created before this timestamp (RFC3339 format). |

#### Response

A JSON array of feed items sorted by `created_at` descending. Each item has a `type` field that determines its shape.

**Common fields on all items:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | String | Item type: `"edit"`, `"comment"`, or `"boost"`. |
| `id` | Integer | Unique ID of the item (event ID, comment ID, or boost ID). |
| `element_id` | Integer | The btcmap element ID. |
| `element_name` | String | Human-readable place name. |
| `created_at` | ISO 8601 datetime | When the item was created. |

**Additional fields for `edit` items:**

| Field | Type | Description |
|-------|------|-------------|
| `user_id` | Integer | ID of the user who made the edit. |
| `user_name` | String | Display name of the user. |
| `action` | String | Edit action: `"create"`, `"update"`, or `"delete"`. |

**Additional fields for `comment` items:**

| Field | Type | Description |
|-------|------|-------------|
| `comment` | String | The comment text. |

**Additional fields for `boost` items:**

| Field | Type | Description |
|-------|------|-------------|
| `duration_days` | Integer | Duration of the boost in days. |

#### Example Response

```json
[
  {
    "type": "edit",
    "id": 4501,
    "user_id": 45,
    "user_name": "Pete",
    "element_id": 678,
    "element_name": "Satoshi Bar",
    "action": "update",
    "created_at": "2026-04-03T12:00:00Z"
  },
  {
    "type": "comment",
    "id": 89,
    "element_id": 678,
    "element_name": "Satoshi Bar",
    "comment": "Great place, accepts lightning!",
    "created_at": "2026-04-03T11:30:00Z"
  },
  {
    "type": "boost",
    "id": 12,
    "element_id": 678,
    "element_name": "Satoshi Bar",
    "duration_days": 30,
    "created_at": "2026-04-03T10:00:00Z"
  }
]
```

#### Example Requests

Get the latest 50 feed items for Thailand:
```
GET /v4/areas/thailand/feed
```

Get the latest 10 items:
```
GET /v4/areas/thailand/feed?limit=10
```

Paginate using the `after` cursor (use `created_at` from the last item of the previous page):
```
GET /v4/areas/thailand/feed?limit=25&after=2026-04-03T10:00:00Z
```

Look up by area ID instead of alias:
```
GET /v4/areas/42/feed
```

#### Pagination Notes

The `after` cursor uses the `created_at` timestamp. In rare cases where multiple items share the exact same `created_at` value, an item could be skipped across page boundaries. This is unlikely in practice since timestamps have microsecond precision.

#### Error Responses

| Status | Description |
|--------|-------------|
| `404` | Area not found. |
| `400` | Invalid `after` timestamp or `limit` value. |
