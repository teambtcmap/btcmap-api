# sync_place_reports

## Description

Pushes every open [`report_place`](report_place.md) to the [teambtcmap/btcmap-data](https://gitea.btcmap.org/teambtcmap/btcmap-data) issue tracker so BTC Map reviewers can triage them. The RPC is idempotent and should be called periodically (for example, from a cron job) until all pending reports have been synced.

For every open report (one that has no `ticket_url` and is not closed/deleted), `sync_place_reports` will:

1. Look up the [`place_import_origin`](get_place_import_origins.md) and skip the report if the origin is unknown or has `gitea_sync_enabled` turned off.
2. Look up the `element` referenced by `place_id` and skip the report if the place no longer exists.
3. Create a Gitea issue labelled with the place-report label (id `903`); when the report `type` is `refused_sats` or `out_of_business`, the removal label (id `904`) is added as well. The returned URL is then saved as the report's `ticket_url`. The origin's `gitea_label_id` is intentionally not applied here — that label only tags new place submissions, not reports against existing places.

For every already-synced report, the RPC re-fetches the linked issue and marks the report `closed_at` whenever the issue state transitions to `closed`.

A short status message is sent to the `#place-import` Matrix room for each created or closed issue so reviewers can react in real time.

## Params

This method takes no parameters.

```json
{}
```

## Result Format

```json
{
  "issues_pending": 12,
  "issues_created": 3,
  "issues_closed": 1
}
```

- `issues_pending`: How many reports remain open after this run (`open_reports - issues_closed`).
- `issues_created`: How many Gitea issues were created on this run.
- `issues_closed`: How many reports were marked `closed_at` because their Gitea issue is now `closed`.

## Allowed Roles

- `root`
- `admin`

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"sync_place_reports","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
