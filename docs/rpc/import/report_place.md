# report_place

## Description

Trusted external sources sometimes need to flag specific problems with a place that already exists on BTC Map: an outdated entry, a missing or broken payment method, a verification check, etc. `report_place` lets those sources record a typed report against a single BTC Map place (an `element`) so the BTC Map team can review and resolve it.

Reports are written to the `place_report` table and carry an optional `extra_fields` payload for any extra context the source wants to surface. The place itself is not modified by this call.

Reports also have a `ticket_url` column that is populated later by an internal background job (which creates the corresponding Gitea issue and writes back its URL). The caller cannot set `ticket_url` directly — it is treated as an internal field.

### Fire-and-forget

`report_place` is fire-and-forget. Every call creates a brand-new issue — resubmitting with the same `(origin, place_id, type)` does **not** update the previous issue. There is no patch/update method; if the source wants to record a follow-up (e.g. a corrected comment), it just calls `report_place` again.

### Well-known types

The `type` field is a free-form lowercase token, but a small set of values has a stable, cross-source meaning and is what BTC Map reviewers expect to see. Trusted sources should prefer one of these whenever it fits:

| Type | Meaning |
|---|---|
| `verified` | The place exists and currently accepts Bitcoin payments. |
| `refused_sats` | An on-site attempt to pay with Bitcoin was refused by the merchant. |
| `out_of_business` | The place has permanently closed or is otherwise no longer operating. |

Other `type` values are allowed and may be useful for source-specific categorisation, but reports using them won't be aggregated or filtered by BTC Map tooling the way the well-known types are.

## Params

```json
{
  "origin": "acme",
  "place_id": 12345,
  "type": "verified",
  "extra_fields": {
    "comment": "had lunch there"
  }
}
```

- `origin`: A unique, lowercase, single-word identifier for the data source. Must match the `name` of a configured entry in the `place_import_origin` table.
- `place_id`: The numeric BTC Map `element.id` the report refers to.
- `type`: A short, lowercase, single-token identifier for the kind of report being filed. See [Well-known types](#well-known-types) for the canonical values; other tokens are accepted but won't be picked up by reviewers' tooling.
- `extra_fields` (optional): A JSON object carrying any extra context the source wants to attach to the report (severity, free-form notes, etc.). The `btcmap-cli` shorthand `--comment "..."` writes its value into `extra_fields.comment`.

## Result Format

```json
{
  "id": 1,
  "origin": "acme",
  "place_id": 12345,
  "type": "verified"
}
```

## Allowed Roles

- root
- admin
- places_source

## Examples

### btcmap-cli

```bash
btcmap-cli place report 12345 \
  --origin acme \
  --type verified \
  --comment "had lunch there"
```

The `--comment` value is folded into `extra_fields.comment` server-side, so the request above is equivalent to passing `--extra-fields '{"comment":"had lunch there"}'` directly. To send richer structured data, use `--extra-fields` instead.

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"report_place","params":{"origin":"acme","place_id":12345,"type":"verified","extra_fields":{"comment":"had lunch there"}},"id":1}' \
  https://api.btcmap.org/rpc
```