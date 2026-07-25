# get_wallets

## Description

Returns the aggregated balance, in satoshis, for the three on-chain wallet buckets configured in the `conf` table: `xpub_spending`, `xpub_donations`, and `xpub_treasury`. Each field may contain a single xpub or a comma-separated list of xpubs; balances are summed across all xpubs in the field.

The wallet balance is fetched from the first reachable electrum server in the `electrum_server` table (ordered by descending `priority`). If the first server fails, the next one is tried and so on. Configure servers via [`add_electrum_server`](electrum_server/add_electrum_server.md).

## Params

```json
{}
```

## Result Format

```json
{
  "spending": 12345,
  "donations": 67890,
  "treasury": 11111
}
```

## Fields

- `spending`: Total balance, in satoshis, of all xpubs listed in `conf.xpub_spending`. `0` if the field is empty.
- `donations`: Total balance, in satoshis, of all xpubs listed in `conf.xpub_donations`. `0` if the field is empty.
- `treasury`: Total balance, in satoshis, of all xpubs listed in `conf.xpub_treasury`. `0` if the field is empty.

All three fields are `0` when every `conf.xpub_*` field is empty, without making any network call.

The method returns an error when at least one xpub is configured but no active `electrum_server` row exists.

## Allowed Roles

- Root
- Admin

## Examples

### curl

```bash
curl --header 'Content-Type: application/json' \
  --header "Authorization: Bearer $ACCESS_TOKEN" \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"get_wallets","params":{},"id":1}' \
  https://api.btcmap.org/rpc
```
