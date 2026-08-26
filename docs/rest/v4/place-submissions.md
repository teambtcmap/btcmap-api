# Place Submissions REST API (v4)

This document describes the endpoint for fetching open place submissions in REST API v4.

A place submission is a record produced by an external import source (e.g. `square`, `coinos`) that proposes adding a new merchant to BTC Map. Submissions are reviewed by editors before they become regular places. The endpoint returns submissions that are still open and have not been revoked.

## Get Open Place Submissions

```bash
curl 'https://api.btcmap.org/v4/place-submissions'
```

Returns the list of place submissions that are not closed and not revoked, ordered by `updated_at` (newest first) and then by `id` (descending).

#### Query Parameters

| Parameter | Type   | Example   | Default | Description                                       |
|-----------|--------|-----------|---------|---------------------------------------------------|
| `source`  | String | `square`  | -       | Optional. Filter by `origin` (e.g. `square`, `coinos`). |

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
    "created_at": "2026-08-25T02:25:35.917Z",
    "updated_at": "2026-08-25T02:35:01.334Z"
  }
]
```

| Field           | Type                | Description                                                                                                  |
|-----------------|---------------------|--------------------------------------------------------------------------------------------------------------|
| `id`            | Number              | Unique identifier of the submission.                                                                          |
| `origin`        | String              | Name of the import source that produced the submission (e.g. `square`, `coinos`).                            |
| `external_id`   | String              | ID assigned by the source system; unique per `origin`.                                                       |
| `lat`           | Number              | Latitude of the proposed place.                                                                              |
| `lon`           | Number              | Longitude of the proposed place.                                                                             |
| `category`      | String              | OSM-style category for the place (e.g. `cafe`, `restaurant`, `bar_club_lounge`).                             |
| `name`          | String              | Display name of the proposed place.                                                                          |
| `extra_fields`  | Object (string→any) | Free-form, source-specific metadata (address, opening hours, icon URL, etc.).                              |
| `ticket_url`    | String or null      | Human-readable URL of the review ticket (e.g. the Gitea issue page) tracking the submission, or `null` if no ticket has been created. Internally the DB stores the Gitea API URL (`…/api/v1/repos/owner/repo/issues/N`); the `/api/v1/repos` segment is stripped on the way out so clients receive a link that opens the issue page in a browser. |
| `revoked`       | Boolean             | `true` if the source has retracted the submission. The endpoint never returns revoked rows.                  |
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
curl --request GET 'https://api.btcmap.org/v4/place-submissions?source=square' | jq
```

Returns only submissions produced by the `square` source. Replace `square` with any other configured import origin (e.g. `coinos`).

##### Empty Result

```bash
curl --request GET 'https://api.btcmap.org/v4/place-submissions?source=does_not_exist'
```

Returns `[]` when the source is unknown or has no open submissions.
