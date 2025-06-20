# get_invoice

## Description

Some paid features generate invoices and this method allows you to check payment status, if you know invoice UUID.

## Params

```json
{
  "uuid": "7fad008b-ebfe-4cdf-98cc-ffe0ee68f024"
}
```

## Result Format

```json
{
  "uuid": "7fad008b-ebfe-4cdf-98cc-ffe0ee68f024",
  "status": "paid"
}
```

## Allowed Roles

- Anon
- User
- Admin
- Root

## Examples

### btcmap-cli

```bash
btcmap-cli rpc get_invoice '{"uuid":"7fad008b-ebfe-4cdf-98cc-ffe0ee68f024"}'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_invoice","params":{"uuid":"7fad008b-ebfe-4cdf-98cc-ffe0ee68f024"},"id":1}' \
  https://api.btcmap.org/rpc
```