
# Feeds API (v3)

Endpoints for retrieving Atom feeds for various activities.

## New Places Feed

Retrieves an Atom feed of newly added places.

```
GET /feeds/new-places
```

### Example Response

Returns an Atom feed XML with entries for new places added to the system.

## New Places Feed for Area

Retrieves an Atom feed of newly added places in a specific area.

```
GET /feeds/new-places/{area_url_alias}
```

### Path Parameters

| Parameter     | Type   | Description |
|---------------|--------|-------------|
| area_url_alias | string | The URL alias of the area |

### Example Response

Returns an Atom feed XML with entries for new places added to the specified area.

## New Comments Feed

Retrieves an Atom feed of new comments.

```
GET /feeds/new-comments
```

### Example Response

Returns an Atom feed XML with entries for new comments added to the system.

## New Comments Feed for Area

Retrieves an Atom feed of new comments for places in a specific area.

```
GET /feeds/new-comments/{area_url_alias}
```

### Path Parameters

| Parameter     | Type   | Description |
|---------------|--------|-------------|
| area_url_alias | string | The URL alias of the area |

### Example Response

Returns an Atom feed XML with entries for new comments on places in the specified area.