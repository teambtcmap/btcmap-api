# delete_event

## Description

Everything flows, everything changes. If an event's location has changed or it's been cancelled, use this method to remove it from BTC Map.

## Params

```json
{
  "id": 1
}
```

## Result Format

```json
{
  "id": 1
}
```

## Allowed Roles

- Root
- Admin
- Event Manager

## Examples

### btcmap-cli

```bash
btcmap-cli delete-event 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"delete_event","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```
