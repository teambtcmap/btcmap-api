# RPC API

The BTCMap RPC API provides a set of methods for interacting with the BTCMap platform.

## Overview

The RPC API is accessed through HTTP POST requests to the endpoint:

```
https://api.btcmap.org/rpc
```

## Method Categories

Methods are grouped by the type of resource they operate on:

- [Element Methods](element-methods.md) - Methods for managing map elements
- [Area Methods](area-methods.md) - Methods for managing geographic areas
- [User Methods](user-methods.md) - Methods for user operations
- [Search Methods](search-methods.md) - Methods for searching various resources

## Authentication and Permissions

Some methods require authentication and specific roles. Each method documentation includes:

- **Required Role**: The role needed to execute the method (if any)
- **Parameters**: The required and optional parameters
- **Response**: The expected response format

## Request Format

RPC requests should be made with the following format:

```json
{
  "id": "1",
  "method": "method_name",
  "params": {
    "param1": "value1",
    "param2": "value2"
  }
}
```

## Response Format

RPC responses will be in the following format:

```json
{
  "id": "1",
  "result": { ... },
  "error": null
}
```

Or in case of an error:

```json
{
  "id": "1",
  "result": null,
  "error": {
    "code": -32000,
    "message": "Error message"
  }
}