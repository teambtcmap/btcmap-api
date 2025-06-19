# create_invoice

## Description

This is a generic method for creating BTC Map invoices. All invoices are real Lightning [BOLT11](https://www.bolt11.org/) payment requests and you should able to pay those invoices with every compatible wallet.

## Params

```json
{
  "amount_sats": 5,
  "description": "donation from anon"
}
```

## Result Format

```json
{
  "uuid": "58c773f7-b32c-460e-8442-8805a7bc2c42",
  "payment_request": "lnbc..."
}
```

## Allowed Roles

- Root

## Examples

### btcmap-cli

```bash
btcmap-cli rpc create_invoice '{"amount_sats":5,"description":"donation from anon"}'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"create_invoice","params":{"amount_sats":5,"description":"donation from anon"},"id":1}' \
  https://api.btcmap.org/rpc
```