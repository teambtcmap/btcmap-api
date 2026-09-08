# Wallet RPC

Methods for managing project onchain wallets.

Each wallet has its own row in the `wallet` table; its `xpub` is scanned by a
background refresher every 5 minutes against the highest-priority reachable
`electrum_server`, and the resulting balance plus recent transactions are
stored back on the row in `cached_balance_sats`, `cached_tx`, and `cached_at`.

- [get_wallets](get_wallets.md) - List wallets with their cached data
- [add_wallet](add_wallet.md) - Add a new wallet
- [update_wallet](update_wallet.md) - Update an existing wallet
- [remove_wallet](remove_wallet.md) - Soft-delete a wallet
