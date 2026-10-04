# Place Submissions REST API (v4)

A place submission is a record that proposes adding a new merchant to BTC Map. Submissions are reviewed by editors before they become regular places.

Submissions produced by external import sources (e.g. `square`, `coinos`) flow through the [`submit_place`](../../rpc/import/submit_place.md) RPC. Submissions produced directly by signed-in BTC Map users flow through the REST endpoint described on this page. Both flavours land in the same `place_submission` table; the REST endpoint always tags the source as `origin: "user"`.

## Available Endpoints

- [Get Open Place Submissions](#get-open-place-submissions)
- [Submit a Place](#submit-a-place)

## Get Open Place Submissions

```bash
curl 'https://api.btcmap.org/v4/place-submissions'
```

Returns the list of place submissions that are not closed and not revoked, ordered by `updated_at` (newest first) and then by `id` (descending). Use this endpoint to inspect both REST and RPC submissions — there is no filter to distinguish between them.

#### Query Parameters

| Parameter | Type   | Example   | Default | Description                                       |
|-----------|--------|-----------|---------|---------------------------------------------------|
| `source`  | String | `square`  | -       | Optional. Filter by `origin` (e.g. `square`, `coinos`, `user`). |

#### Response

```json
[
  {
    "id": 18108,
    "origin": "square",
    "external_id": "LV1DBWP3XAD0F",
    "lat": 40.0429202,
    "lon": -76.3631632,
    "category": "beauty_and_barber_shops",
    "name": "Vibrissae LLC",
    "extra_fields": {
      "address": "101 Rohrerstown Rd Ste 141 Lancaster PA 17603-2274 US",
      "description": "Spa inspired pet grooming...",
      "opening_hours": "Mo,We,Th,Fr 10:00-18:00; Tu 10:00-17:00; Sa 10:00-16:00; Su 09:00-17:00",
      "icon_url": "https://square-web-production-f.squarecdn.com/files/.../original.jpeg",
      "last_updated": "2026-08-25T02:25:48.649336835Z"
    },
    "ticket_url": "https://gitea.btcmap.org/teambtcmap/btcmap-data/issues/24272",
    "revoked": false,
    "submitted_by": 7,
    "created_at": "2026-08-25T02:25:35.917Z",
    "updated_at": "2026-08-25T02:35:01.334Z"
  }
]
```

| Field           | Type                | Description                                                                                                  |
|-----------------|---------------------|--------------------------------------------------------------------------------------------------------------|
| `id`            | Number              | Unique identifier of the submission.                                                                          |
| `origin`        | String              | Name of the import source that produced the submission. `user` for submissions created via this REST endpoint; `square`, `coinos`, `btcpayserver`, etc. for submissions created via the RPC. |
| `external_id`   | String              | Source-system identifier for the merchant; unique per `origin`. Always populated for RPC submissions (the source system picks it). REST submissions always have a synthetic value and clients should ignore it.                                                       |
| `lat`           | Number              | Latitude of the proposed place.                                                                              |
| `lon`           | Number              | Longitude of the proposed place.                                                                              |
| `category`      | String              | OSM-style category for the place (e.g. `cafe`, `restaurant`, `bar_club_lounge`).                             |
| `name`          | String              | Display name of the proposed place.                                                                          |
| `extra_fields`  | Object (string→any) | Free-form, source-specific metadata (address, opening hours, icon URL, etc.).                              |
| `ticket_url`    | String or null      | Human-readable URL of the review ticket (e.g. the Gitea issue page) tracking the submission, or `null` if no ticket has been created. Internally the DB stores the Gitea API URL (`…/api/v1/repos/owner/repo/issues/N`); the `/api/v1/repos` segment is stripped on the way out so clients receive a link that opens the issue page in a browser. |
| `revoked`       | Boolean             | `true` if the source has retracted the submission. The endpoint never returns revoked rows.                  |
| `submitted_by`  | Number or null      | `id` of the user who originally created the submission. REST submissions always carry a value here (the authenticated caller); RPC submissions from external import sources may have `null` for legacy rows. |
| `created_at`    | ISO 8601 datetime   | When the submission was first received.                                                                      |
| `updated_at`    | ISO 8601 datetime   | When the submission was last updated.                                                                        |
| `closed_at`     | ISO 8601 datetime   | When the submission was closed. Omitted while the submission is still open.                                  |
| `deleted_at`    | ISO 8601 datetime   | When the submission was soft-deleted. Omitted while the submission is still open.                            |

#### Special Behavior

- Only submissions with `closed_at IS NULL` and `revoked = false` are returned.
- The endpoint is public (no auth required). Because the data is intended for public review by community editors, do not assume the absence of a submission here is permanent; rows appear and disappear as editors work through them.
- The list is unsorted with respect to submission source. Use `?source=...` to filter to a single origin.

#### Examples

##### Fetch All Open Submissions

```bash
curl --request GET 'https://api.btcmap.org/v4/place-submissions' | jq
```

##### Filter by Source

```bash
curl --request GET 'https://api.btcmap.org/v4/place-submissions?source=user' | jq
```

Returns only submissions produced by signed-in BTC Map users (i.e. via this REST endpoint). Replace `user` with `square` or any other RPC origin to filter further.

##### Empty Result

```bash
curl --request GET 'https://api.btcmap.org/v4/place-submissions?source=does_not_exist'
```

Returns `[]` when the source is unknown or has no open submissions.

## Submit a Place

Most BTC Map locations are sourced from [OpenStreetMap](https://www.openstreetmap.org/about). However, instant mass-importing data into OSM is impossible due to its [lengthy](https://wiki.openstreetmap.org/wiki/Import), uncertain approval process.

Signed-in BTC Map users can use this endpoint to add a place directly without waiting for the OSM merge pipeline. Editors will review the submission and, if approved, merge it into OSM. This endpoint is the user-facing counterpart of the [`submit_place`](../../rpc/import/submit_place.md) RPC and is restricted to the `user` origin — external import sources should keep using the RPC.

### Authentication

Requires a Bearer token obtained from the [Auth](auth.md) endpoint. The token does not need any special role; any signed-in user can submit a place.

Each call creates a fresh submission. If you want to edit a submission you already filed, file a new one — the REST endpoint does not patch previous submissions in place. To revoke a submission, use [`revoke_submitted_place`](../../rpc/import/revoke_submitted_place.md) instead.

### Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/place-submissions' \
     --header "Authorization: Bearer $ACCESS_TOKEN" \
     --header 'Content-Type: application/json' \
     --data '{
       "lat": 18.2649,
       "lon": 98.5013,
       "category": "cafe",
       "name": "Satoshi Cafe",
       "extra_fields": {
         "website": "https://example.com"
       }
     }'
```

#### Request Body

| Field          | Type                | Required | Description                                                                                              |
|----------------|---------------------|----------|----------------------------------------------------------------------------------------------------------|
| `lat`          | Number              | Yes      | Latitude of the merchant. Must be between -90 and 90.                                                   |
| `lon`          | Number              | Yes      | Longitude of the merchant. Must be between -180 and 180.                                                |
| `category`     | String              | Yes      | OSM-style category for the place (e.g. `cafe`, `restaurant`, `bar_club_lounge`).                       |
| `name`         | String              | Yes      | Display name of the merchant.                                                                            |
| `extra_fields` | Object (string→any) | No       | Free-form additional fields (address, opening hours, icon URL, etc.). See the [Places](places.md#field-selection) docs for the supported fields; custom fields are also allowed. |

### Response

```json
{
  "id": 18108,
  "origin": "user"
}
```

| Field    | Type   | Description                                                                                  |
|----------|--------|----------------------------------------------------------------------------------------------|
| `id`     | Number | Unique identifier of the newly created submission. Use it to look the row up via the [Get Open Place Submissions](#get-open-place-submissions) endpoint. |
| `origin` | String | Always `"user"` for this endpoint.                                                           |

The created submission appears in [Get Open Place Submissions](#get-open-place-submissions) until editors close or revoke it.

### Error Responses

| Status | Meaning                                                                                                          |
|--------|------------------------------------------------------------------------------------------------------------------|
| 400    | One or more fields are invalid (e.g. `lat` outside [-90, 90], empty `category`/`name`).                          |
| 401    | Missing or invalid Bearer token.                                                                                  |
| 500    | Database error. Contact the BTC Map team if this persists.                                                       |
