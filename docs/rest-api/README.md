# REST API

The BTCMap API provides REST endpoints for retrieving data about elements, events, users, areas, reports, element comments, and element issues.

## API Versions

The API has multiple versions:
- [v2](v2/README.md): Original API with basic endpoints
- [v3](v3/README.md): Enhanced API with additional resources and improved filtering
- [v4](v4/README.md): Latest version with newer endpoints and advanced features

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
- [Reports API](reports.md) - Access user reports about elements
- [Element Comments API](element-comments.md)
- [Element Issues API](element-issues.md)
- [Feeds API](feeds.md) - Access Atom feeds for various activities
- [Area Elements API](area-elements.md) - Access elements within geographic areas