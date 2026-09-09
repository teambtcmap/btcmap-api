# Place Reports REST API (v4)

A place report is a record that flags a problem with an existing place (e.g. "this place has closed", "the listed Lightning node is offline", "the place no longer accepts Bitcoin"). Editors review reports and act on them.

Reports produced by external import sources (e.g. `square`, `coinos`) flow through the [`report_place`](../../rpc/import/report_place.md) RPC. Reports filed directly by signed-in BTC Map users flow through the REST endpoint described on this page. Both flavours land in the same `place_report` table; the REST endpoint always tags the source as `origin: "user"`.

## Available Endpoints

- [Report a Place](#report-a-place)

## Report a Place

Signed-in BTC Map users can use this endpoint to flag an issue with a place without going through an external import pipeline. Editors will see the report on their review queue. This endpoint is the user-facing counterpart of the [`report_place`](../../rpc/import/report_place.md) RPC and is restricted to the `user` origin — external import sources should keep using the RPC.

### Authentication

Requires a Bearer token obtained from the [Auth](auth.md) endpoint. The token does not need any special role; any signed-in user can file a report.

Each call creates a fresh report. If a place still needs attention, file another report — the REST endpoint does not patch previous reports in place.

### Request

```bash
curl --request POST \
     --url 'https://api.btcmap.org/v4/place-reports' \
     --header "Authorization: Bearer $ACCESS_TOKEN" \
     --header 'Content-Type: application/json' \
     --data '{
       "place_id": 42,
       "type": "verification",
       "extra_fields": {
         "comment": "Looks closed when I drove past on Sunday"
       }
     }'
```

#### Request Body

| Field          | Type                | Required | Description                                                                                              |
|----------------|---------------------|----------|----------------------------------------------------------------------------------------------------------|
| `place_id`     | Number              | Yes      | Database ID of the place being reported.                                                                 |
| `type`         | String              | Yes      | Report type (e.g. `verification`, `missing_payment_method`). Free-form; editors triage by `type`.       |
| `extra_fields` | Object (string→any) | No       | Free-form additional context (e.g. `comment`, `payment_methods_seen`). Custom keys are allowed.         |

### Response

```json
{
  "id": 18108,
  "origin": "user"
}
```

| Field    | Type   | Description                                                                                  |
|----------|--------|----------------------------------------------------------------------------------------------|
| `id`     | Number | Unique identifier of the newly created report. Use it to look the row up.                    |
| `origin` | String | Always `"user"` for this endpoint.                                                           |

The `submitted_by` column is populated server-side from the authenticated user; clients do not send it.

### Error Responses

| Status | Meaning                                                                                                          |
|--------|------------------------------------------------------------------------------------------------------------------|
| 401    | Missing or invalid Bearer token.                                                                                  |
| 500    | Database error, or the `user` import origin is not configured on this deployment. Contact the BTC Map team if this persists. |
