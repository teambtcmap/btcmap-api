# get_submitted_place

## Description

You can use this method to look up the previously submitted data.

## Params

```json
{
  "id": 3
}
```

**OR**

```json
{
  "origin": "acme",
  "external_id": "15"
}
```

## Result Format

```json
{
  "id": 1,
  "origin": "acme",
  "external_id": "15",
  "lat": 1.23,
  "lon": 4.56,
  "category": "bar",
  "name": "Satoshi Pub",
  "extra_fields": {
    "website": "https://satoshi.pub"
  },
  "revoked": true
}
```

## Allowed Roles

- root
- admin
- places_source

## Examples

### btcmap-cli

```bash
btcmap-cli get-submitted-place acme:15
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_submitted_place","params":{"origin":"acme","external_id":"15"},"id":1}' \
  https://api.btcmap.org/rpc
```
