# BTC Map Analytics RPC

The RPC API provides a [JSON-RPC 2.0](https://www.jsonrpc.org/specification) interface that can be used to generate various analytical reports.

## Methods

- [dashboard](dashboard.md): Get a high-level analytics snapshot (place activity, imports by origin, log stats incl. unique IPs per platform, disk usage, LND balances, and recent sync runs over 1/7/30 day windows).
- [get_report](get_report.md): Get analytics report comparing place statistics between two dates.
- [get_daily_infra_report](get_daily_infra_report.md): Get daily infrastructure report.
- [get_top_clients](get_top_clients.md): Get top API consumers.
- [get_request_log](get_request_log.md): Get recent API requests.