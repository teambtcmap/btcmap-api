# get_event

## Description

Retreives an event by id

## Params

```json
{
  "id": 1
}
```

## Result Format

```json
{
  "id": 1,
  "lat": 7.8812324,
  "lon": 98.3884695,
  "name": "Phuket Bitcoin Meetup",
  "website": "https://www.meetup.com/phuket-bitcoin-meetup/events/310120143/",
  "starts_at": "2025-08-29T19:00:00+07:00",
  "ends_at": null
}
```

## Allowed Roles

- Root
- Admin
- User

## Examples

### btcmap-cli

```bash
btcmap-cli get-event 1
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_event","params":{"id":1},"id":1}' \
  https://api.btcmap.org/rpc
```
