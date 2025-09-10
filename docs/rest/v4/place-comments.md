# Place Comments REST API (v4)

Comments add valuable context to BTC Map places. They are also anonymous and free of spam, thanks to Great Satswall. We will probably add optional user authentication in the future to reduce payment friction, but the strictly anonymous 'pay-per-comment' model is the only one we currently support.

## Available Endpoints

- [Get a Comment Quote](#get-a-comment-quote)
- [Order Comment](#order-comment)

### Get a Comment Quote

Comments are a paid feature. Always get the latest quote and show it to the user before they commit to the purchase.

#### Example Request

```bash
curl https://api.btcmap.org/v4/place-comments/quote
```

#### Example Output

```json
{
  "quote_sat": 500
}
```

### Order Comment

Once the user has seen the quote and typed their comment, you can use this endpoint to submit the comment intent and fetch the actual Lightning invoice.

#### Example Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/place-comments' \
     --header 'Content-Type: application/json' \
     --data '{"place_id": "12345", "comment": "Amazing view!"}'
```

#### Request Parameters

| Parameter | Type   | Example                      | Comments                                                 |
|-----------|--------|------------------------------|----------------------------------------------------------|
| place_id  | string | 12345                        | -                                                        |
| comment   | string | Bitcoiner owned, I recommend | Sensible length limits apply, don't submit War and Peace |

#### Example Response

```json
{
  "invoice_id": "dd79bb72-6535-4ada-a683-88b6e8550f14",
  "invoice": "lnbc..."
}
```