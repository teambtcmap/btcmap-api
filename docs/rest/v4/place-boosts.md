# Place Boosts REST API (v4)

Boosting merchants helps them get seen. It's how we pay the bills without relying on sketchy ads or other shady ways to make money.

## Available Endpoints

- [Get a Boost Quote](#get-a-boost-quote)
- [Order Boost](#order-boost)

### Get a Boost Quote

Boosts are a paid feature. Always get the latest quote and show it to the user before they commit to the purchase.

#### Example Request

```bash
curl https://api.btcmap.org/v4/place-boosts/quote
```

#### Example Output

```json
{
  "quote_30d_sat": 5000,
  "quote_90d_sat": 10000,
  "quote_365d_sat": 30000
}
```

### Order Boost

Once the user has seen the quote and decided on a boost duration (30, 90, or 365 days), you can use this endpoint to submit the boost intent and fetch the actual Lightning invoice.

#### Example Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/place-boosts' \
     --header "Content-Type: application/json" \
     --data '{"place_id": "12345", "days": 30}'
```

#### Request Parameters

| Parameter | Type   | Example | Comments                           |
|-----------|--------|---------|------------------------------------|
| place_id  | string | 12345   | -                                  |
| days      | number | 30      | Currelty limited to 30, 90 and 365 |

#### Example Response

```json
{
  "invoice_id": "dd79bb72-6535-4ada-a683-88b6e8550f14",
  "invoice": "lnbc..."
}
```