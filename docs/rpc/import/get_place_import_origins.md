# get_place_import_origins

## Description

Lists every import origin currently configured on the server. Each entry describes the vendor identifier, whether submissions from that origin should be synced to Gitea, and the optional Gitea label ID that should be attached to the imported issue.

## Params

This method takes no parameters.

## Result Format

```json
[
  {
    "name": "bitcoin-jungle",
    "gitea_sync_enabled": true,
    "gitea_label_id": 1552
  },
  {
    "name": "btcpayserver",
    "gitea_sync_enabled": true,
    "gitea_label_id": 1538
  },
  {
    "name": "coinos",
    "gitea_sync_enabled": false,
    "gitea_label_id": null
  },
  {
    "name": "square",
    "gitea_sync_enabled": true,
    "gitea_label_id": 1307
  },
  {
    "name": "square-test",
    "gitea_sync_enabled": true,
    "gitea_label_id": 1551
  }
]
```

## Allowed Roles

- root
- admin

## Examples

### btcmap-cli

```bash
btcmap-cli place-import list-origins
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_place_import_origins","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```