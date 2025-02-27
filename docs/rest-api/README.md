
# REST API

The BTCMap API provides REST endpoints for retrieving data about elements, events, users, areas, reports, element comments, and element issues.

## API Versions

The API has multiple versions:
- v2: Older version of the API
- v3: Current stable version
- v4: Latest version with newer endpoints

## Authentication

Some endpoints require authentication. For those endpoints, you need to include authentication credentials in the request headers.

## Error Handling

All API endpoints follow a consistent error handling pattern. When an error occurs, the API will return an appropriate HTTP status code along with error details.

Common status codes:
- 400: Bad Request - The request was invalid
- 401: Unauthorized - Authentication failed
- 404: Not Found - The requested resource was not found
- 500: Internal Server Error - Something went wrong on the server

## Available Endpoints

- [Elements API](elements.md)
- [Events API](events.md)
- [Users API](users.md)
- [Areas API](areas.md)
- [Reports API](reports.md)
- [Element Comments API](element-comments.md)
- [Element Issues API](element-issues.md)
- [Feeds API](feeds.md)
