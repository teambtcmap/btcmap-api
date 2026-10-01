# Place Images REST API (v4)

A place image is a photo attached to a place. Today the only producer is the
optional photo evidence on [Place Reports](place-reports.md), which stores
uploads against the place with `type = "report"`. The store is place-scoped: a
place can have many images, and further `type` values may be added later.

Images live in the `image.db` `place` table and are served publicly.

## Available Endpoints

- [List Place Images](#list-place-images)
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
| `type`    | String | `report` | -       | Optional filter. When omitted, images of every type for the place are returned.          |

#### Response

```json
[
  {
    "id": 3,
    "place_id": 42,
    "type": "report",
    "width": 1024,
    "height": 768,
    "size_bytes": 184320,
    "created_at": "2026-10-01T04:22:53.706Z"
  }
]
```

| Field        | Type   | Description                                                                                  |
|--------------|--------|----------------------------------------------------------------------------------------------|
| `id`         | Number | Unique identifier of the image. Use it to fetch the bytes from [Get Place Image](#get-place-image). |
| `place_id`   | Number | Database ID of the place the image is attached to.                                            |
| `type`       | String | Image type, e.g. `report`.                                                                    |
| `width`      | Number | Image width in pixels.                                                                        |
| `height`     | Number | Image height in pixels.                                                                       |
| `size_bytes` | Number | Size of the stored bytes.                                                                     |
| `created_at` | String | RFC 3339 timestamp of when the image was stored.                                              |

The response is an empty array when the place has no images.

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

The response `Content-Type` is set from the stored bytes. Report evidence is
restricted to PNG, JPEG and WebP at upload time, so these images are always
raster.

#### Errors

| Status | Meaning                                                                                       |
|--------|-----------------------------------------------------------------------------------------------|
| 400    | `id` too long, or `w`/`h` set to `0`.                                                          |
| 404    | The place does not exist, the image does not exist, or the image belongs to a different place. |
