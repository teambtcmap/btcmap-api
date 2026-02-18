# AGENTS.md - BTC Map API Developer Guide

This document provides guidelines for agents working on the btcmap-api codebase.

## Project Overview

BTC Map API is a Rust web service built with actix-web that provides a REST and RPC APIs for managing Bitcoin adoption data in meatspace. It uses SQLite for persistence.

## Build & Test Commands

### Building
```bash
cargo build --release # Production build
cargo build # Debug build
```

### Running
```bash
cargo run # Run the server (binds to 127.0.0.1:8000)
```

### Testing
```bash
cargo test # Run all tests
cargo test --verbose # Run all tests with verbose output
cargo test <test_name> # Run a single test by name
cargo test -- --nocapture # Run tests with stdout/stderr output
```

### Linting & Formatting
```bash
cargo clippy # Run linter (includes many warnings)
cargo clippy -- -D warnings # Treat warnings as errors
cargo fmt # Format code
cargo fmt --check # Check formatting without modifying
```

### Running a Specific Test
To run a single test, use the test name filter:
```bash
cargo test get_empty_array
cargo test get_not_empty_array
```

## Code Structure

```
src/
├── main.rs          # Application entry point
├── error.rs         # Main Error enum with From impls
├── db/              # Database layer (queries, blocking_queries, schema)
│   ├── mod.rs
│   ├── element/
│   ├── user/
│   ├── area/
│   └── ...
├── service/         # Business logic layer
│   ├── mod.rs
│   ├── element.rs
│   ├── user.rs
│   └── ...
├── rest/            # RWST HTTP handlers (v2, v3, v4 APIs)
│   ├── mod.rs
│   ├── error.rs     # REST API error types
│   └── v4/
├── rpc/             # RPC handlers
├── og/              # OpenGraph image generation
└── feed/            # Atom/RSS feeds
```

## Code Style Guidelines

### Naming Conventions
- **Modules**: lowercase with underscores (e.g., `element_comment`)
- **Types/Structs**: PascalCase (e.g., `User`, `RestApiError`)
- **Functions**: snake_case (e.g., `select_by_id`, `get_places`)
- **Constants**: SCREAMING_SNAKE_CASE for static values
- **Database tables**: singular, lowercase (e.g., `user`, `element`)

### Imports Organization
Order imports by category with blank lines between groups:
1. `use crate::` (internal modules)
2. External crate imports (actix-web, serde, etc.)
3. `use std::` imports

```rust
use crate::db;
use crate::db::element::schema::Element;
use crate::rest::error::RestApiError;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
```

### Error Handling
- Use the custom `Error` enum from `src/error.rs`
- Implement `From` traits for automatic error conversion
- For REST endpoints, wrap errors with `RestApiError`

```rust
// In service/handler code
.map_err(|_| RestApiError::database())?
```

### Database Patterns
The codebase uses a two-layer pattern:
- `queries.rs` - Async functions using deadpool SQLite pool
- `blocking_queries.rs` - Synchronous functions running on blocking threads

```rust
// queries.rs (async)
pub async fn select_by_id(id: i64, pool: &Pool) -> Result<User> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

// blocking_queries.rs (sync)
pub fn select_by_id(id: i64, conn: &mut Connection) -> Result<User> {
    // ... rusqlite queries
}
```

Full database schema is always available in schema.sql, use it as reference and don't try to make up non-existing tables and fields.

### REST Handler Patterns
Handlers return `RestResult<T>` which is `Result<Json<T>, RestApiError>`:

```rust
#[get("")]
pub async fn get(args: Query<GetListArgs>, pool: Data<Pool>) -> Res<Vec<JsonObject>> {
    // ... implementation
    Ok(Json(items))
}
```

### Testing Patterns
Tests are inline using `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod test {
    use crate::db::test::pool;
    use actix_web::test::{self, TestRequest};
    use actix_web::web::scope;
    use actix_web::App;

    #[test]
    async fn test_name() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::my_handler)),
        ).await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }
}
```

Use `db::test::pool()` for in-memory SQLite test databases.

### SQL Conventions
- Use parameterized queries to prevent SQL injection
- Table names are singular (e.g., `user`, not `users`)
- Use migrations in `src/db/migration.rs`

### Configuration
- Server binds to `127.0.0.1:8000` (hardcoded in main.rs)
- Database stored in data directory (configurable via `data_dir_file_path`)
- Logging controlled via `RUST_LOG` env var (defaults to "info")
- Release builds use native CPU optimization (via .cargo/config.toml)

### Key Dependencies
- **actix-web**: HTTP server
- **rusqlite + deadpool-sqlite**: SQLite database with async pool
- **serde**: JSON serialization
- **reqwest**: HTTP client
- **time**: Date/time handling
- **tracing**: Logging
- **geo + geojson**: Geographic data
