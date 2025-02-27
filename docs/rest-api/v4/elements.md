# Elements API (v4)

This document describes the endpoints for interacting with elements in API v4.

## Available Endpoints

- [Get Elements List](#get-elements-list)
- [Get Element by ID](#get-element-by-id)

### Get Elements List

```
GET /v4/elements
```

Retrieves a list of elements that have been updated since a specific time.

#### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `updated_since` | ISO 8601 datetime | **Required**. Filter elements updated since this time (RFC3339 format). |
| `limit` | Integer | **Required**. Limit the number of elements returned. |
| `include_deleted` | Boolean | Optional. Whether to include deleted elements. Default is `true`. |
| `include_tag` | String | Optional. Can be specified multiple times to include specific tags in the response. |

### Incremental Sync Approach

The `/v4/elements` endpoint is designed for efficient incremental synchronization. Clients should:

1. Store the timestamp of their last sync locally
2. Request elements that have been updated since that timestamp using the `updated_since` parameter
3. Process only the changes since the last sync
4. Update their local timestamp for the next sync

This approach minimizes data transfer and processing requirements, making it ideal for mobile applications and other bandwidth-constrained environments.

#### Example Incremental Sync Flow:

```
// Initial sync - store returned timestamp
GET /v4/elements?updated_since=2020-01-01T00:00:00Z&limit=1000

// Subsequent sync - use timestamp from previous response
GET /v4/elements?updated_since=2023-09-15T14:30:45Z&limit=1000
```

### Limits

The following limits apply to the `/v4/elements` endpoint:

- **Rate Limit**: 60 requests per minute per IP address
- **Maximum Limit Parameter**: 1000 elements per request
- **Default Limit**: 100 elements if not specified
- **Maximum Response Size**: 10MB

Exceeding these limits will result in a `429 Too Many Requests` or `413 Payload Too Large` error response.

#### Tag Selection

The `include_tag` parameter allows you to request specific tags to be included in the response, which can improve performance for large requests. You can specify the parameter multiple times to include multiple tags.

Available tags include:

```
name                    // Element name
phone                   // Contact phone number
website                 // Website URL
check_date              // Date the element was last checked
survey:date             // Date the element was surveyed
check_date:currency:XBT // Bitcoin acceptance check date
addr:street             // Street address
addr:housenumber        // Street address house number
contact:website         // Contact website
opening_hours           // Business hours
contact:phone           // Contact phone
contact:email           // Contact email
contact:twitter         // Twitter handle
contact:instagram       // Instagram handle
contact:facebook        // Facebook page
contact:line            // Line contact
btcmap:icon             // Icon identifier
btcmap:boost:expires    // Boost expiration date
btcmap:osm:type         // OpenStreetMap element type
btcmap:osm:id           // OpenStreetMap ID
btcmap:osm:url          // OpenStreetMap URL
btcmap:created_at       // Creation timestamp
btcmap:updated_at       // Update timestamp
btcmap:deleted_at       // Deletion timestamp
btcmap:lat              // Latitude
btcmap:lon              // Longitude
```

##### Examples:

Basic request for active merchants with location and name:
```
GET /v4/elements?include_deleted=false&include_tag=btcmap:lat&include_tag=btcmap:lon&include_tag=name
```

Request with additional contact information:
```
GET /v4/elements?include_deleted=false&include_tag=btcmap:lat&include_tag=btcmap:lon&include_tag=name&include_tag=contact:website&include_tag=contact:phone
```

Request with detailed address information:
```
GET /v4/elements?include_deleted=false&include_tag=name&include_tag=addr:street&include_tag=addr:housenumber
```

#### Response

```json
[
  {
    "id": 123456,
    "osm_type": "node",
    "osm_id": 123456,
    "geolocation": {
      "latitude": 40.7128,
      "longitude": -74.0060
    },
    "tags": {
      "name": "Bitcoin Coffee",
      "amenity": "cafe",
      "currency:XBT": "yes"
    },
    "issues": [
      {
        "id": 1,
        "type": "closed",
        "created_at": "2023-02-10T12:00:00Z"
      }
    ],
    "updated_at": "2023-01-15T00:00:00Z"
  }
]
```

#### Example Request

```
GET /v4/elements?updated_since=2023-01-01T00:00:00Z&limit=10
```

### Get Element by ID

```
GET /v4/elements/{id}
```

Retrieves a specific element by its ID.

#### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | Integer | **Required**. The element ID |

#### Response

```json
{
  "id": 123456,
  "osm_type": "node",
  "osm_id": 123456,
  "geolocation": {
    "latitude": 40.7128,
    "longitude": -74.0060
  },
  "tags": {
    "name": "Bitcoin Coffee",
    "amenity": "cafe",
    "currency:XBT": "yes"
  },
  "issues": [
    {
      "id": 1,
      "type": "closed",
      "created_at": "2023-02-10T12:00:00Z"
    }
  ],
  "updated_at": "2023-01-15T00:00:00Z"
}
```

#### Example Request

```
GET /v4/elements/123456