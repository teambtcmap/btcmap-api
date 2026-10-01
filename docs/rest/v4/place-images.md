# Place Images REST API (v4)

A place image is a photo attached to a place. There are two producers:

- Optional photo evidence on [Place Reports](place-reports.md), stored against
  the place with `type = "report"`.
- Direct uploads from signed-in users, described on this page and stored with
  `type = "user"`.

The store is place-scoped: a place can have many images, and further `type`
values may be added later. When the uploader is known (both of the producers
above), their user ID is recorded in `created_by`.

Images live in the `image.db` `place` table and are served publicly.

## Available Endpoints

- [List Place Images](#list-place-images)
- [Add Place Image](#add-place-image)
- [Get Place Image](#get-place-image)

### List Place Images

Returns metadata for the images attached to a place — no bytes. Newest first.

```bash
curl 'https://api.btcmap.org/v4/places/42/images'
```

#### Path Parameters

| Parameter | Type   | Example                   | Description                                                              |
|-----------|--------|---------------------------|--------------------------------------------------------------------------|
| `id`      | String | `42` or `node:1234567890` | **Required**. Place ID (numeric) or an OSM reference (`node:…`, `way:…`, `relation:…`). |

#### Query Parameters

| Parameter | Type   | Example  | Default | Description                                                                              |
|-----------|--------|----------|---------|------------------------------------------------------------------------------------------|
| `type`    | String | `user`   | -       | Optional filter. When omitted, images of every type for the place are returned. Common values are `user` (direct uploads) and `report` (report evidence). |

#### Response

```json
[
  {
    "id": 3,
    "place_id": 42,
    "type": "user",
    "width": 1024,
    "height": 768,
    "size_bytes": 184320,
    "created_at": "2026-10-01T04:22:53.706Z",
    "created_by": 17
  }
]
```

| Field        | Type   | Description                                                                                  |
|--------------|--------|----------------------------------------------------------------------------------------------|
| `id`         | Number | Unique identifier of the image. Use it to fetch the bytes from [Get Place Image](#get-place-image). |
| `place_id`   | Number | Database ID of the place the image is attached to.                                            |
| `type`       | String | Image type, e.g. `user` or `report`.                                                          |
| `width`      | Number | Image width in pixels.                                                                        |
| `height`     | Number | Image height in pixels.                                                                       |
| `size_bytes` | Number | Size of the stored bytes.                                                                     |
| `created_at` | String | RFC 3339 timestamp of when the image was stored.                                              |
| `created_by` | Number | Optional. ID of the signed-in user who uploaded the image, omitted when unknown.               |

The response is an empty array when the place has no images.

### Add Place Image

Signed-in users can upload a photo for a place directly, without filing a
[report](place-reports.md). This is the endpoint for community-contributed place
photos (storefronts, interiors, signage). Uploads are stored against the place
with `type = "user"` and attributed to the caller in `created_by`.

#### Authentication

Requires a Bearer token obtained from the [Auth](auth.md) endpoint. Any
signed-in user may upload; no special role is required.

#### Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/places/42/images' \
     --header "Authorization: Bearer $ACCESS_TOKEN" \
     --header 'Content-Type: application/json' \
     --data '{ "data_base64": "<base64-encoded image>" }'
```

#### Path Parameters

| Parameter | Type   | Example                   | Description                                                              |
|-----------|--------|---------------------------|--------------------------------------------------------------------------|
| `id`      | String | `42` or `node:1234567890` | **Required**. Place ID (numeric) or an OSM reference (`node:…`, `way:…`, `relation:…`). |

#### Request Body

| Field         | Type   | Required | Description                                                                                              |
|---------------|--------|----------|----------------------------------------------------------------------------------------------------------|
| `data_base64` | String | Yes      | Base64-encoded raster image. Subject to the same constraints as report evidence: PNG, JPEG or WebP only; at most 10 MB decoded; at most 20,000 px on either side. |

The image is fully decoded and validated before anything is stored, so an
invalid upload returns `400` without creating a row.

#### Response

Returns the stored image, in the same shape as an item from
[List Place Images](#list-place-images):

```json
{
  "id": 4,
  "place_id": 42,
  "type": "user",
  "width": 1024,
  "height": 768,
  "size_bytes": 184320,
  "created_at": "2026-10-01T04:22:53.706Z",
  "created_by": 17
}
```

#### Errors

| Status | Meaning                                                                                                          |
|--------|------------------------------------------------------------------------------------------------------------------|
| 400    | Invalid request body, or the image is not valid base64 / too large / not a supported raster format.               |
| 401    | Missing or invalid Bearer token.                                                                                  |
| 404    | The place does not exist.                                                                                         |
| 500    | Database error. Contact the BTC Map team if this persists.                                                        |

### Get Place Image

Serves the stored bytes. Supports on-the-fly raster resizing, so clients can
request exactly the dimensions they need without downloading a multi-megabyte
master asset.

```bash
curl -o evidence.jpg 'https://api.btcmap.org/v4/places/42/images/3'
```

#### Path Parameters

| Parameter  | Type   | Example | Description                                                                              |
|------------|--------|---------|------------------------------------------------------------------------------------------|
| `id`       | String | `42`    | **Required**. Place ID (numeric) or an OSM reference. Must match the image's place.      |
| `image_id` | Number | `3`     | **Required**. Image ID from [List Place Images](#list-place-images).                     |

#### Query Parameters

| Parameter | Type    | Example | Default       | Description                                              |
|-----------|---------|---------|---------------|----------------------------------------------------------|
| `w`       | Integer | `256`   | source width  | Maximum output width in pixels.                          |
| `h`       | Integer | `256`   | source height | Maximum output height in pixels.                         |

`w` and `h` must each be greater than `0` when provided; a value of `0`
returns `400 invalid_input`. Resizing behaves exactly like
[Get Area Image](areas.md#get-area-image): bounds are maximums, the image is
never upscaled, and resizing is skipped (original bytes returned) when the
source already fits.

The response `Content-Type` is set from the stored bytes. Uploaded images are
restricted to PNG, JPEG and WebP at upload time, so these images are always
raster.

#### Errors

| Status | Meaning                                                                                       |
|--------|-----------------------------------------------------------------------------------------------|
| 400    | `id` too long, or `w`/`h` set to `0`.                                                          |
| 404    | The place does not exist, the image does not exist, or the image belongs to a different place. |
