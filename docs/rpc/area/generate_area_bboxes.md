# generate_area_bboxes

## Description

Recomputes the bounding box (`bbox_west`, `bbox_south`, `bbox_east`,
`bbox_north`) for every area in the database from its stored GeoJSON
geometry and writes the result back to the `area` row. Any drift between
the geometry and the persisted bbox columns is logged at WARN level with
the area's alias.

The response reports only the number of areas whose bbox was corrected;
areas whose stored bbox already matches the geometry are left untouched.

## Params

Empty params object:

```json
{}
```

## Result Format

```json
{
  "areas_affected": 3
}
```

## Allowed Roles

- Root

## Examples

### btcmap-cli

```bash
btcmap-cli area generate-bboxes
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"generate_area_bboxes","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```