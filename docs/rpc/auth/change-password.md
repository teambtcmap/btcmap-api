# change_password

## Description

Sometimes a BTC Map admin sets up an account for you with a username and a password. If you didn't make that password yourself, you should change it.

Also, use this if you think your current password sucks or might have been leaked.

Just so you know, BTC Map never stores your actual password. We only keep a hashed version of it on the backend.

That doesn't mean you can safely reuse passwords from other services though. If the backend is compromised and you're logging in, third parties could still intercept your password. We recommend generating a unique, strong password for your BTC Map account.

## Params

```json
{
  "username": "Satoshi",
  "old_password": "oldpwd",
  "new_password": "verystrongnewpwd!"
}
```

## Result

```json
{
  "time_ms": 300
}
```

## Examples

### btcmap-cli

```bash
btcmap-cli change-password Satoshi oldpwd verystrongnewpwd!
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"change_password","params":{"username":"Satoshi","old_password":"oldpwd","new_password":"verystrongnewpwd!"},"id":1}' \
  https://api.btcmap.org/rpc
```