# generate_areas_elements_mapping

## Description

Recomputes the area-to-element mapping for every element currently in the
database. The response lists each element whose membership changed, along
with the area ids that were added or removed. Run this after a bulk
import of new areas, or to recover from a suspected drift between elements
and areas.

## Params

Empty params object:

```json
{}
```

## Result Format

```json
{
  "affected_elements": [
    {
      "element_id": 42,
      "element_osm_url": "https://www.openstreetmap.org/node/42",
      "added_areas": [49],
      "removed_areas": []
    }
  ]
}
```

## Allowed Roles

- Root

## Examples

### btcmap-cli

```bash
btcmap-cli generate-areas-elements-mapping
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"generate_areas_elements_mapping","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
