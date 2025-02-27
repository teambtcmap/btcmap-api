# BTCMap API v4

The v4 API is the latest version of the API with the most advanced features. This version introduces element issues tracking and provides additional functionality for elements.

## Currently Implemented Endpoints

- [Elements](elements.md) - Access and manage map elements
- [Element Issues](element-issues.md) - Retrieve and manage issues associated with elements

## Planned Endpoints

The following endpoints are planned for v4 but have not been implemented yet. Currently, you can use their v3 equivalents:

- [Users](users.md) - Access user information
- [Areas](areas.md) - Access geographical area information
- [Events](events.md) - Access events related to map elements
- [Feeds](feeds.md) - Access Atom feeds for various activities

## Authentication

Some endpoints require authentication. For those endpoints, you need to include authentication credentials in the request headers.

## Error Handling

All API endpoints follow a consistent error handling pattern. When an error occurs, the API will return an appropriate HTTP status code along with error details.

Common status codes:
- 400: Bad Request - The request was invalid
- 401: Unauthorized - Authentication failed
- 404: Not Found - The requested resource was not found
- 500: Internal Server Error - Something went wrong on the server