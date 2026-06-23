# change_password

## Description

Use this endpoint if you need to update your password. User passwords are encrypted at rest using the Argon2 KDF.

## Params

| Field          | Type   | Required | Description                                                          |
| -------------- | ------ | -------- | -------------------------------------------------------------------- |
| `username`     | string | yes      | The account name to update the password for                          |
| `old_password` | string | yes      | The current account password                                         |
| `new_password` | string | yes      | The new account password; must be 12 to 64 characters                 |

```json
{
  "username": "satoshi",
  "old_password": "oldpwd",
  "new_password": "newpwd"
}
```

## Result

```json
{
  "changed": true
}
```

## Errors

| Message                                                | Cause                                                          |
| ------------------------------------------------------ | -------------------------------------------------------------- |
| `New password is too short, use at least 12 chars`     | `new_password` is shorter than 12 characters                   |
| `New password is too long, use at most 64 chars`       | `new_password` is longer than 64 characters                    |
| `Incorrect username or password`                       | The username does not exist or `old_password` is wrong         |
| `Unexpected error, please contact administrator`       | An internal error occurred                                     |

## Examples

### btcmap-cli

```bash
btcmap-cli change-password --user satoshi --old oldpwd --new newpwd
```

### curl

```bash
curl --header 'Content-Type: application/json' \
  --request POST \
  --data '{"jsonrpc":"2.0","method":"change_password","params":{"username":"Satoshi","old_password":"oldpwd","new_password":"newpwd"},"id":1}' \
  https://api.btcmap.org/rpc
```