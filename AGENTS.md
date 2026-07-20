# AGENTS.md - BTC Map API Developer Guide

This document provides guidelines for agents working on the btcmap-api codebase.

## Project Overview

BTC Map API is a Rust web service built with actix-web that provides a REST and RPC APIs for managing Bitcoin adoption data in meatspace. It uses SQLite for persistence.

## Build & Test Commands

### Building
```bash
cargo build
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

### Linting
```bash
cargo clippy -- -D warnings
```

### Formatting
```bash
cargo fmt
```

### Running a Specific Test
To run a single test, use the test name filter:
```bash
cargo test get_empty_array
cargo test get_not_empty_array
```

### Pre-commit checklist
Always run these before committing:
```bash
cargo fmt            # Format code
cargo clippy -- -D warnings  # Lint (must pass with zero warnings)
cargo test           # Run tests
```

## Local Server Workflow

When working on REST or RPC endpoints, test them locally to validate changes. Some endpoints require auth token.

### Databases
The running server uses the real local DBs at `$HOME/.local/share/btcmap/`: `main.db`, `log.db`, `image.db`. Unit tests use in-memory pools and never touch them.

### Build once
```bash
cargo build
```
Debug build is enough for local poking.

### Start the server in the background
Run the compiled binary directly so the recorded PID belongs to the server rather than to `cargo`:

```bash
(
  set -eu
  pid_file=/tmp/opencode/btcmap-server.pid
  log_file=/tmp/opencode/btcmap-server.log

  if [ -s "$pid_file" ]; then
    old_pid="$(cat "$pid_file")"
    if [[ "$old_pid" =~ ^[1-9][0-9]*$ ]] &&
      kill -0 "$old_pid" 2>/dev/null &&
      [[ "$(ps -p "$old_pid" -o comm=)" == *btcmap-api* ]]; then
      printf 'Server already running (PID %s)\n' "$old_pid"
      exit 0
    fi
    rm -f "$pid_file"
  fi

  nohup env RUST_LOG="${RUST_LOG:-info}" ./target/debug/btcmap-api \
    >"$log_file" 2>&1 </dev/null &
  pid=$!
  printf '%s\n' "$pid" >"$pid_file"

  for _ in {1..60}; do
    if curl -fsS --max-time 1 \
      'http://127.0.0.1:8000/v2/elements?limit=1' >/dev/null; then
      printf 'Server ready (PID %s)\n' "$pid"
      exit 0
    fi
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done

  kill -TERM "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  rm -f "$pid_file"
  tail -n 50 "$log_file" >&2
  exit 1
)
```

The readiness probe verifies that the HTTP server and database are usable. If startup fails or times out, the command stops the process, removes the PID file, and prints the latest log output.

### Stop the server
Send `SIGTERM` first so Actix can shut down gracefully. After ten seconds, force termination if needed:

```bash
(
  set -eu
  pid_file=/tmp/opencode/btcmap-server.pid

  if [ ! -s "$pid_file" ]; then
    printf 'Server is not running\n'
    exit 0
  fi

  pid="$(cat "$pid_file")"
  if ! [[ "$pid" =~ ^[1-9][0-9]*$ ]] ||
    ! kill -0 "$pid" 2>/dev/null ||
    [[ "$(ps -p "$pid" -o comm=)" != *btcmap-api* ]]; then
    rm -f "$pid_file"
    printf 'Removed stale PID file\n'
    exit 0
  fi

  kill -TERM "$pid"
  for _ in {1..100}; do
    if ! kill -0 "$pid" 2>/dev/null; then
      rm -f "$pid_file"
      exit 0
    fi
    sleep 0.1
  done

  kill -KILL "$pid"
  rm -f "$pid_file"
)
```

### Create a temporary admin token
Some endpoints require `Bearer` auth with a token that has a priviledged role. Mint a test token if not present in the local `main.db`:

```bash
sqlite3 $HOME/.local/share/btcmap/main.db <<'SQL'
INSERT INTO access_token (user_id, name, secret, roles)
VALUES (
  (SELECT id FROM user WHERE roles LIKE '%root%' LIMIT 1),
  'agent-probe',
  'opencode',
  '["root"]'
);
SQL
```
Pick a user with the `root` role and a `secret` string that is static and set to "opencode". The `secret` becomes the bearer token.

### Call an RPC which requires auth
```bash
curl -sS \
  -X POST http://127.0.0.1:8000/rpc \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer opencode' \
  -d '{"jsonrpc":"2.0","id":"1","method":"get_wallets"}'
```
- Endpoint: `POST /rpc` (not `/v2/rpc` or `/v3/rpc` — there's no version prefix).
- Method names are `snake_case` (e.g. `get_wallets`, `get_element`, `dashboard`).
- Response is JSON-RPC 2.0: `result` on success, `error` with `code`/`message`/`data` on failure.
- For slow calls, add `--max-time 180` to `curl`.

### Verifying changes against the real DB
Useful when you want to see if a patch actually works on production-shaped data (real element counts, real wallet history, etc.) rather than the empty in-memory test DB. The pattern is:
1. Make the code change.
2. `cargo build` (debug is faster).
3. Restart the server (kill + relaunch as above).
4. Hit the endpoint with `curl`, read the response.
5. Tail `/tmp/opencode/btcmap-server.log` for warnings/errors.
6. Iterate.

### Endpoints cheat sheet
- `POST /rpc` — JSON-RPC, the main admin/control surface.
- `GET /v2/elements`, `/v2/elements/{id}`, `/v2/areas`, `/v2/areas/{id}`, `/v2/reports`, `/v2/users` — read-only public API.
- `GET /v3/...` and `GET /v4/...` — same data, newer shape; see `docs/rest/v3/` and `docs/rest/v4/`.
- `GET /feeds/...` — Atom/RSS feeds.
- `GET /og/element/{id}` — OpenGraph image for a place.

### Rust version
The project does not pin its Rust version. Contributors should use a recent stable Rust toolchain with `rustfmt` and `clippy` components installed.

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

**All SQL queries must go through the blocking_queries/queries layer. Never embed raw SQL in REST or RPC handlers.** If you need a new query, add it to the appropriate `blocking_queries.rs` file and create an async wrapper in `queries.rs`. Each table has its own subfolder under `src/db/main/` (e.g., `element_event/`, `element_comment/`, `area_element/`).

### REST Handler Patterns
Handlers return `RestResult<T>` which is `Result<Json<T>, RestApiError>`:

```rust
#[get("")]
pub async fn get(args: Query<GetListArgs>, pool: Data<MainPool>) -> Res<Vec<JsonObject>> {
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
- Database stored in ~/.local/share/btcmap
- Logging controlled via `RUST_LOG` env var (defaults to "info")
- Release builds use native CPU optimization (via .cargo/config.toml)

### Temporary Files
For scratch files, logs, PIDs, or any other transient artifacts outside the workspace, always use `/tmp/opencode` — this path is pre-approved and won't trigger permission prompts. Do NOT use bare `/tmp/...` paths. Examples: `/tmp/opencode/btcmap-server.log`, `/tmp/opencode/btcmap-server.pid`.

### Key Dependencies
- **actix-web**: HTTP server
- **rusqlite + deadpool-sqlite**: SQLite database with async pool
- **serde**: JSON serialization
- **reqwest**: HTTP client
- **time**: Date/time handling
- **tracing**: Logging
- **geo + geojson**: Geographic data
