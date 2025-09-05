# report_imported_merchant

## Description

If you previously imported a merchant and are highly confident it no longer accepts sats, call this method to notify us. We will react accordingly.

If the merchant in question has not yet been upstreamed, we will simply delete it from our merge queue. If the merchant is already on OSM, we will perform manual checks to comply with OSM requirements. This method abstracts away those nuances, allowing you to focus on importing quality Bitcoin merchants while ensuring that those who have changed their mind about Bitcoin acceptance are properly removed.

## Params

```json
{
  "origin": "acme",
  "external_id": "15"
}
```

The following fields are mandatory, as they represent the minimum required to display merchants on the map meaningfully:

- `origin`: A unique, lowercase, single-word identifier for the data source.
- `external_id`: The identifier for the merchant within your source database. Please send numeric identifiers as strings.

## Result Format

```json
{
  "id": "acme:15"
}
```

## Allowed Roles

- Root
- Admin
- Importer

## Examples

### btcmap-cli

```bash
btcmap-cli report-imported-merchant --origin 'acme' --external-id 15
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"report_imported_merchant","params":{"origin":"acme","external_id":"15"},"id":1}' \
  https://api.btcmap.org/rpc
```
