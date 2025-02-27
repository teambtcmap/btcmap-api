
# Feeds API

The Feeds API provides Atom feeds for various activities on the platform.

## Endpoints

### GET /feeds/new-places

Provides an Atom feed of newly added places.

#### Example Request

```
GET /feeds/new-places
```

#### Example Response

```xml
<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>BTC Map - New Places</title>
  <link href="https://api.btcmap.org/feeds/new-places" rel="self" />
  <updated>2023-05-15T14:30:45Z</updated>
  <id>https://api.btcmap.org/feeds/new-places</id>
  <entry>
    <title>New Bitcoin Coffee Shop</title>
    <link href="https://btcmap.org/place/123456" />
    <id>https://btcmap.org/place/123456</id>
    <updated>2023-05-15T14:30:45Z</updated>
    <content type="html">
      A new Bitcoin-accepting coffee shop has been added to the map.
    </content>
    <author>
      <name>JohnDoe</name>
    </author>
  </entry>
  <!-- Additional entries -->
</feed>
```
