# sync_submitted_places

## Description

Pushes every pending place submission to the [teambtcmap/btcmap-data](https://gitea.btcmap.org/teambtcmap/btcmap-data) issue tracker so BTC Map reviewers can verify the imported place in OSM. The RPC is idempotent and should be called periodically (for example, from a cron job) until all pending submissions and revocations have been synced.

For every open submission (one that has no `ticket_url`, is not closed and is not revoked), `sync_submitted_places` will:

1. Look up the [`place_import_origin`](get_place_import_origins.md) and skip the submission if the origin is unknown or has `gitea_sync_enabled` turned off.
2. Create a Gitea issue labelled with the location-submission label (id `901`) and the origin's `gitea_label_id`, titled with the country and community tags of the areas the place falls in. The returned URL is saved as the submission's `ticket_url`.

For every already-synced submission, the RPC re-fetches the linked issue and marks the submission as `closed_at` whenever the issue state transitions to `closed`.

## Revocations

Submissions that were [revoked](revoke_submitted_place.md) are picked up once they have a `ticket_url`, so a revocation is never lost to a race with issue creation. The revocation label (id `904`) doubles as the marker of an already-processed revocation, and revocation processing happens after issue processing so tickets created on this run are covered on the next one.

For every revoked submission with a ticket, `sync_submitted_places` will:

1. Skip the ticket if it already carries the location-removal label (id `904`).
2. Relabel the ticket with the location-removal label, dropping the location-submission label and keeping the origin label.
3. Close the ticket and add a comment when the ticket is still open, since no reviewer has actioned it yet.
4. Reopen the ticket when it is already closed, since a reviewer has already processed the submission and needs to see the revocation.

A short status message is sent to the `#place-import` Matrix room for each created, closed or reopened issue so reviewers can react in real time.

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
  "issues_closed": 1,
  "revocations_processed": 2
}
```

- `issues_pending`: How many submissions remain open after this run (`open_submissions - issues_closed`).
- `issues_created`: How many Gitea issues were created on this run.
- `issues_closed`: How many submissions were marked `closed_at` because their Gitea issue is now `closed`.
- `revocations_processed`: How many revoked submissions had their Gitea ticket closed or reopened on this run.

## Allowed Roles

- `root`
- `admin`

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer ***" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"sync_submitted_places","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
