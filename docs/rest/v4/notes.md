# Notes REST API (v4)

Notes are free-form text pins that signed-in users drop at a coordinate. They
are a pure overlay: a note references no place, area or other map entity, only
its author. This makes notes a good fit for things that are not (yet) a place —
"ATM inside the bar here", "good coffee, no Lightning yet" — and for personal
reminders.

Every note has a `public` flag:

- **Private** (default) — visible only to its author.
- **Public** — readable by anyone, and returned by the geo search.

The author can flip a note between private and public at any time. Making a note
private again removes it from search immediately.

## Available Endpoints

- [Create Note](#create-note)
- [List My Notes](#list-my-notes)
- [Get Note](#get-note)
- [Update Note](#update-note)
- [Delete Note](#delete-note)
- [Search Public Notes](#search-public-notes)

## Create Note

Creates a note owned by the caller. Requires a Bearer token obtained from the
[Auth](auth.md) endpoint. Any signed-in user may create notes.

Notes are **private unless `public` is set to `true`**.

### Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/notes' \
     --header "Authorization: Bearer $ACCESS_TOKEN" \
     --header 'Content-Type: application/json' \
     --data '{
       "lat": 53.5503,
       "lon": 9.9921,
       "text": "ATM is inside, ask at the bar",
       "public": true
     }'
```

#### Request Body

| Field    | Type    | Required | Description                                                                     |
|----------|---------|----------|---------------------------------------------------------------------------------|
| `lat`    | Number  | Yes      | Latitude, within `[-90, 90]`.                                                   |
| `lon`    | Number  | Yes      | Longitude, within `[-180, 180]`.                                                |
| `text`   | String  | Yes      | Note body. Trimmed, non-empty, at most 2000 characters.                          |
| `public` | Boolean | No       | Whether the note is visible to everyone. Defaults to `false` (private).          |

### Response

```json
{
  "id": 7,
  "lat": 53.5503,
  "lon": 9.9921,
  "text": "ATM is inside, ask at the bar",
  "public": true,
  "author": { "id": 17, "name": "satoshi" },
  "created_at": "2026-10-07T12:00:00Z",
  "updated_at": "2026-10-07T12:00:00Z"
}
```

| Field        | Type    | Description                                                              |
|--------------|---------|--------------------------------------------------------------------------|
| `id`         | Number  | Unique note identifier.                                                  |
| `lat`        | Number  | Latitude.                                                                |
| `lon`        | Number  | Longitude.                                                               |
| `text`       | String  | Note body.                                                               |
| `public`     | Boolean | Whether the note is public.                                              |
| `author`     | Object  | Note owner as `{ "id": Number, "name": String }`.                        |
| `created_at` | String  | RFC 3339 creation timestamp.                                             |
| `updated_at` | String  | RFC 3339 timestamp of the last change (text, visibility or deletion).    |

### Errors

| Status | Meaning                                                   |
|--------|-----------------------------------------------------------|
| 400    | Invalid coordinates, or empty / oversized text.            |
| 401    | Missing or invalid Bearer token.                           |
| 500    | Database error. Contact the BTC Map team.                  |

## List My Notes

Returns every note belonging to the authenticated user, public **and** private.

```bash
curl 'https://api.btcmap.org/v4/users/me/notes' \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Requires authentication.** See [Auth](auth.md) for details.

### Query Parameters

| Parameter         | Type            | Default                    | Description                                                                  |
|-------------------|-----------------|----------------------------|------------------------------------------------------------------------------|
| `updated_since`   | ISO 8601 datetime | `1970-01-01T00:00:00Z`   | Return only notes changed after this instant, so a client can refresh its cache. |
| `include_deleted` | Boolean         | `false`                    | Include notes the user has deleted, so a cached client can evict them.        |
| `limit`           | Integer         | `100`                      | Capped at `500`.                                                             |

The response is an array of [note objects](#response). Deleted notes carry a
`deleted_at` field.

Since this endpoint is owner-scoped, notes are returned whatever their
visibility. To show a user's own pins alongside public ones on a map, combine
this list with [Search Public Notes](#search-public-notes) and de-duplicate by
`id` on the client.

### Errors

| Status | Meaning                                        |
|--------|------------------------------------------------|
| 401    | Missing or invalid Bearer token.               |
| 500    | Database error. Contact the BTC Map team.       |

## Get Note

Fetches a single note.

```bash
curl 'https://api.btcmap.org/v4/notes/7'
```

- A **public** note is returned to anyone.
- A **private** note is only returned to its owner; everyone else receives
  `404` so its existence is not leaked.

Authentication is optional. When a Bearer token is supplied it is used to check
ownership of a private note.

### Errors

| Status | Meaning                                                     |
|--------|-------------------------------------------------------------|
| 404    | The note does not exist, is deleted, or is private and not yours. |
| 500    | Database error. Contact the BTC Map team.                    |

## Update Note

Edits a note and/or changes its visibility. Only the owner may update a note.

```bash
curl --request PATCH \
     --url 'https://api.btcmap.org/v4/notes/7' \
     --header "Authorization: Bearer $ACCESS_TOKEN" \
     --header 'Content-Type: application/json' \
     --data '{ "text": "ATM moved to the back", "public": false }'
```

**Requires authentication.** Only the note's author can update it; anyone else
receives `404`.

### Request Body

At least one of `text` or `public` must be provided. Omitted fields keep their
current value.

| Field    | Type    | Description                                             |
|----------|---------|---------------------------------------------------------|
| `text`   | String  | New note body. Trimmed, non-empty, at most 2000 characters. |
| `public` | Boolean | New visibility.                                         |

The response is the updated [note object](#response).

### Errors

| Status | Meaning                                                        |
|--------|----------------------------------------------------------------|
| 400    | Neither `text` nor `public` was provided, or `text` is empty / oversized. |
| 401    | Missing or invalid Bearer token.                                |
| 404    | The note does not exist, is deleted, or belongs to another user. |
| 500    | Database error. Contact the BTC Map team.                       |

## Delete Note

Soft-deletes the caller's own note. The row is retained with `deleted_at` set so
owner clients can sync the removal; it is never returned by search again.

```bash
curl --request DELETE \
  --url 'https://api.btcmap.org/v4/notes/7' \
  --header "Authorization: Bearer $ACCESS_TOKEN"
```

**Requires authentication.** Only the owner may delete a note.

The response is the deleted [note object](#response), with `deleted_at` set.

### Errors

| Status | Meaning                                                        |
|--------|----------------------------------------------------------------|
| 401    | Missing or invalid Bearer token.                                |
| 404    | The note does not exist, is deleted, or belongs to another user. |
| 500    | Database error. Contact the BTC Map team.                       |

## Search Public Notes

Returns public notes around a point, for drawing notes on a map. Private notes
are never returned, and no authentication is required.

```bash
curl 'https://api.btcmap.org/v4/notes/search?lat=53.55&lon=9.99&radius_km=5'
```

### Query Parameters

| Parameter   | Type    | Default | Description                                                       |
|-------------|---------|---------|-------------------------------------------------------------------|
| `lat`       | Number  | -       | **Required.** Center latitude, within `[-90, 90]`.                 |
| `lon`       | Number  | -       | **Required.** Center longitude, within `[-180, 180]`.              |
| `radius_km` | Number  | `10`    | Search radius in kilometers. Must be positive; capped at `100`.    |
| `limit`     | Integer | `100`   | Capped at `500`.                                                   |

### Response

An array of public [note objects](#response), nearest first. Search responses
additionally carry `distance_km`, the distance from the search center:

```json
[
  {
    "id": 7,
    "lat": 53.5503,
    "lon": 9.9921,
    "text": "ATM is inside, ask at the bar",
    "public": true,
    "author": { "id": 17, "name": "satoshi" },
    "created_at": "2026-10-07T12:00:00Z",
    "updated_at": "2026-10-07T12:00:00Z",
    "distance_km": 0.08
  }
]
```

### Errors

| Status | Meaning                                                                |
|--------|------------------------------------------------------------------------|
| 400    | Missing or invalid `lat`/`lon`, or `radius_km` is not positive.          |
| 500    | Database error. Contact the BTC Map team.                               |
