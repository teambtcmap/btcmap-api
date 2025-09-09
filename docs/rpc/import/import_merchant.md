# import_merchant

## Description

Most BTC Map merchants are sourced from [OpenStreetMap](https://www.openstreetmap.org/about). However, mass-importing data into OSM is nearly impossible due to its [lengthy](https://wiki.openstreetmap.org/wiki/Import), uncertain approval process.

This RPC offers trusted external sources (major franchises, PoS providers, etc.) the ability to get on BTC Map instantly, which benefits BTC Map users and API consumers, including popular Bitcoin wallets. All imported data will also be processed by BTC Map editors and merged into OSM eventually. The merger timeline is unpredictable, as it depends on many factors beyond our control. This method allows BTC Map users to skip the wait while also making it easy for various Bitcoin merchant data sources to open-source their data and promote their merchants.

## Params

```json
{
  "origin": "acme",
  "external_id": "15",
  "lat": 18.2649,
  "lon": 98.5013,
  "category": "cafe",
  "name": "Satoshi Cafe",
  "extra_fields": {
    "website": "https://example.com"
  }
}
```

The following fields are mandatory, as they represent the minimum required to display merchants on the map meaningfully:

- `origin`: A unique, lowercase, single-word identifier for the data source.
- `external_id`: The identifier for the merchant within your source database. Please send numeric identifiers as strings.
- `lat`: The merchant's latitude. Must be reasonably accurate.
- `lon`: The merchant's longitude. Must be reasonably accurate.
- `category`: The merchant's category. Use a short, single-word (if possible), lowercase identifier. We will map your categories to our icons.
- `name`: The merchant's name.

Additionally, a `extra_fields` object is available:

- `extra_fields`: A JSON object containing a set of additional fields for our review and potential inclusion in OSM. You can see the full list of supported fields [here](https://github.com/teambtcmap/btcmap-api/blob/master/docs/rest/v4/places.md#field-selection), but custom fields are also allowed if they are considered important for the merchant in question.

## Result Format

```json
{
  "id": "acme:15"
}
```

## Allowed Roles

- root
- admin
- merchant_source

## Examples

### btcmap-cli

```bash
btcmap-cli import-merchant --origin 'acme' \
  --external-id 15 \
  --lat 18.2649 \
  --lon 98.5013 \
  --category 'cafe' \
  --name 'Satoshi Cafe'
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"import_merchant","params":{"origin":"acme","external_id":"15",lat":18.2649,"lon":98.5013,"category":"cafe","name":"Satoshi Cafe"},"id":1}' \
  https://api.btcmap.org/rpc
```
