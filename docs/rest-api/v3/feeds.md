
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
# Feeds API (v3)

The Feeds API provides access to various activity feeds in Atom format.

## Endpoints

### GET /feeds/new-comments

Provides an Atom feed of the latest comments on elements.

#### Response Format

Atom XML feed containing the most recent comments with the following information:
- Comment content
- Author information (if available)
- Timestamp
- Link to the associated element

#### Example Usage

```
curl https://api.btcmap.org/feeds/new-comments
```

This endpoint is designed to be consumed by feed readers and services that support the Atom format.
