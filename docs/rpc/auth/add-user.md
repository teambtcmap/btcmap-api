# add_user

## Description

BTC Map doesn’t make you sign up, but if you do, you’ll get some extra perks. Right now, most users are admins, but anyone can join if they want!

## Params

```json
{
  "name": "Satoshi",
  "password": "SuperSecurePassword"
}
```

## Result

```json
{
  "name": "Satoshi",
  "roles": [
    "user"
  ]
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli add-user Satoshi SuperSecurePassword
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"add_user","params":{"name":"Satoshi","password":"SuperSecurePassword"},"id":1}' \
  https://api.btcmap.org/rpc
```