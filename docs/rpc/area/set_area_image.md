# set_area_image

## Description

Sets the image for an area. The image bytes are uploaded as a base64-encoded
payload; the format is detected from the bytes server-side and supports SVG,
PNG, WebP, and JPEG. The image is written to `static.btcmap.org` and also
cached in `image.db` so it can be served from the BTC Map image CDN.

The `image_type` parameter selects the image slot — it defaults to `square`,
and the slot is exposed as the legacy tag `icon:{image_type}` (e.g.
`icon:square`, `icon:wide`). Uploading a new image for the same slot replaces
the previous one.

## Params

```json
{
  "area_id": "bangkok",
  "image_base64": "iVBORw0KGgoAAAANSUhEUgAA...<truncated base64 payload>...AAAElFTkSuQmCC",
  "image_type": "wide"
}
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `area_id` | string \| integer | yes | Numeric area id or `url_alias` tag |
| `image_base64` | string | yes | Standard base64 encoding of the image bytes |
| `image_type` | string | no | Image slot to update; defaults to `square`. Maps to the `icon:{image_type}` legacy tag. |

## Result Format

The updated area, identical in shape to [get_area](get_area.md):

```json
{
  "id": 123,
  "tags": {
    "url_alias": "bangkok",
    "name": "Bangkok",
    "icon:square": "https://static.btcmap.org/images/areas/123_square.png",
    "icon:wide": "https://static.btcmap.org/images/areas/123_wide.png"
  },
  "created_at": "2024-01-15T12:34:56Z",
  "updated_at": "2024-06-30T08:00:00Z",
  "deleted_at": null
}
```

## Allowed Roles

- Root
- Admin

## Examples

### btcmap-cli

The CLI reads the image from a file path and base64-encodes it for you:

```bash
btcmap-cli set-area-image --area bangkok --image ./bangkok.png
btcmap-cli set-area-image --area bangkok --type wide --image ./bangkok-wide.png
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"set_area_image","params":{"area_id":"bangkok","image_base64":"iVBORw0KGgoAAAANSUhEUgAA...AAAElFTkSuQmCC","image_type":"wide"},"id":1}' \
  https://api.btcmap.org/rpc
```
