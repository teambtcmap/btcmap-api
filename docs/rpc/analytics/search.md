# search

## Description

Performs a case-insensitive substring search across areas, places and events, returning matching records sorted by source (areas first, then places, then events). When the optional `type` filter is provided, only records of that source are returned.

## Params

```json
{
  "query": "berlin",
  "type": "place"
}
```

| Field  | Type   | Required | Description                                                                                       |
| ------ | ------ | -------- | ------------------------------------------------------------------------------------------------- |
| query  | string | yes      | The search string matched against area names, place names and event names (case-insensitive).    |
| type   | string | no       | Restricts the result set to a single source. Accepts `area`, `place` or `event`. Defaults to all. |

## Result Format

```json
[
  {
    "name": "Berlin",
    "type": "area",
    "id": 42
  },
  {
    "name": "Berlin Bitcoin Meetup",
    "type": "place",
    "id": 1337
  },
  {
    "name": "Bitcoin Berlin Conference",
    "type": "event",
    "id": 9
  }
]
```

| Field | Type    | Description                                                              |
| ----- | ------- | ------------------------------------------------------------------------ |
| name  | string  | Display name of the matched record.                                      |
| type  | string  | Source of the record: `area`, `place` or `event`.                        |
| id    | integer | Database id of the matched area, place or event.                        |

## Allowed Roles

- Root
- Admin
- Event Manager

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"search","params":{"query":"berlin"},"id":1}' \
  https://api.btcmap.org/rpc
```

Search only places:

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"search","params":{"query":"berlin","type":"place"},"id":1}' \
  https://api.btcmap.org/rpc
```