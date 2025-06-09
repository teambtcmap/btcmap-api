
# BTCMap REST API

The BTCMap REST API provides HTTP-based access to BTC Map data.

## Base URL

All REST API endpoints are relative to the base URL: `https://api.btcmap.org/`

## API Versioning

The API is versioned with the paths `/v2/`, `/v3/`, and `/v4/`. When a new incompatible API version is released, the old versions remain available for backwards compatibility.

## Available Endpoints

- **Elements ([v2](v2/elements.md)|[v3](v3/elements.md)|[v4](v4/elements.md))** - Access map elements (locations) data

- **Element Issues ([v4](v4/element-issues.md))** - Manage issues related to elements

- **Element Comments ([v3](v3/element-comments.md))** - Manage comments on elements

- **Users ([v2](v2/users.md)|[v3](v3/users.md)|[v4](v4/users.md))** - Access user data

- **Areas ([v2](v2/areas.md)|[v3](v3/areas.md)|[v4](v4/areas.md))** - Access geographic areas data

- **Events ([v2](v2/events.md)|[v3](v3/events.md)|[v4](v4/events.md))** - Access events data

- **Reports ([v2](v2/reports.md)|[v3](v3/reports.md))** - Access reports data

- **Feeds ([v3](v3/feeds.md)|[v4](v4/feeds.md))** - Access feed data

- **Area Elements ([v3](v3/area-elements.md))** - Access relationships between areas and elements

## Authentication

Most REST API endpoints are publicly accessible without authentication. However, some administrative endpoints may require authentication.

## Rate Limiting

The API is rate-limited to protect against abuse. Please be respectful with your API calls.

## Response Format

All API responses are in JSON format unless otherwise specified. Successful responses typically return a 200 OK status code, while failures return an appropriate HTTP error code.
