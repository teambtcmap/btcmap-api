
# Feeds API (v4)

The Feeds API provides Atom feeds for various activities on the BTCMap platform.

## Endpoints

### Get New Places Feed

```
GET /v4/feeds/new-places
```

Retrieves an Atom feed of newly created places across the entire platform.

#### Response

Returns an Atom XML feed containing information about newly created places.

### Get New Places for Area Feed

```
GET /v4/feeds/new-places/{area}
```

Retrieves an Atom feed of newly created places within a specific area.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `area` | String | The area ID or alias |

#### Response

Returns an Atom XML feed containing information about newly created places in the specified area.

### Get New Comments Feed

```
GET /v4/feeds/new-comments
```

Retrieves an Atom feed of new comments across the entire platform.

#### Response

Returns an Atom XML feed containing information about new comments.

### Get New Comments for Area Feed

```
GET /v4/feeds/new-comments/{area}
```

Retrieves an Atom feed of new comments within a specific area.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `area` | String | The area ID or alias |

#### Response

Returns an Atom XML feed containing information about new comments in the specified area.

## Examples

### Get new places feed

```
GET /v4/feeds/new-places
```

### Get new places feed for Berlin

```
GET /v4/feeds/new-places/berlin
```

### Get new comments feed

```
GET /v4/feeds/new-comments
```

### Get new comments feed for Berlin

```
GET /v4/feeds/new-comments/berlin
```
