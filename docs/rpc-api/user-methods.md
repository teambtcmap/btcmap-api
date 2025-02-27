
# User RPC Methods

This page documents all RPC methods related to users.

## GetUserActivity

Retrieves activity data for a user.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_user_activity",
  "params": {
    "user_id": "user_id"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "user": {
      "id": "user_id",
      "name": "User Name"
    },
    "activity": {
      "comments": 50,
      "elements_added": 20,
      "areas_contributed": 5,
      "last_active": "2023-01-01T00:00:00Z"
    },
    "recent_comments": [
      {
        "id": "comment_id",
        "element_id": "element_id",
        "content": "Comment content",
        "created_at": "2023-01-01T00:00:00Z"
      }
    ]
  },
  "id": 1
}
```

## SetUserTag

Sets a tag for a user. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "set_user_tag",
  "params": {
    "user_id": "user_id",
    "tag": "tag_name"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

## RemoveUserTag

Removes a tag from a user. Requires admin authentication.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "remove_user_tag",
  "params": {
    "user_id": "user_id",
    "tag": "tag_name"
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

## GetMostActiveUsers

Retrieves a list of the most active users.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "get_most_active_users",
  "params": {
    "limit": 10
  },
  "id": 1
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "users": [
      {
        "id": "user_id",
        "name": "User Name",
        "activity_score": 95,
        "comments": 50,
        "elements_added": 20
      }
    ]
  },
  "id": 1
}
```
