# AGENTS.md - BTC Map API Developer Guide

This document provides guidelines for agents working on the btcmap-api codebase.

## Project Overview

BTC Map API is a Rust web service built with actix-web that provides a REST and RPC APIs for managing Bitcoin adoption data in meatspace. It uses SQLite for persistence.

## Commit and Push Policy

**Never commit or push unless the user explicitly instructs it in their most recent prompt.** Even when changes look complete, lint passes, and tests are green, do not run `git commit`, `git push`, `git commit --amend`, or any force-push/rebase on the user's behalf without a direct instruction in the last message. If a commit or push seems like the obvious next step, surface the suggestion in the response and wait for confirmation — do not execute it preemptively.

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

### Pre-commit checklist
Always run these before committing:
```bash
cargo fmt            # Format code
cargo clippy -- -D warnings  # Lint (must pass with zero warnings)
cargo test           # Run tests
```

## Local Server Workflow

Use this when you need a running server to exercise admin-only RPC methods (e.g. `get_wallets`, `dashboard`, `sync_elements`) end-to-end instead of through unit tests.

### Databases
The running server uses the real local DBs at `$HOME/.local/share/btcmap/` (configured by `data_dir_file_path`): `main.db`, `log.db`, `image.db`. Unit tests use in-memory pools and never touch them.

### Build once
```bash
cargo build
```
Debug build is enough for local poking. Release only if you're measuring performance or timing RPC calls (a debug build of `get_wallets` is ~5x slower because of unoptimized serde and crypto paths).

### Start the server in the background
The server blocks on stdin/stdout when run in the foreground, which hangs the shell. Detach it with `setsid` and redirect all streams:
```bash
setsid bash -c 'ELECTRUM_URL=ssl://electrum.blockstream.info:50002 \
                RUST_LOG=info \
                ./target/debug/btcmap-api \
                > /tmp/btcmap-server.log 2>&1 & \
                echo $! > /tmp/btcmap-server.pid; disown' \
  < /dev/null > /dev/null 2>&1 &

sleep 3
kill -0 "$(cat /tmp/btcmap-server.pid)" && echo "alive" || echo "dead"
```
- `setsid` + `disown` + redirected streams = the process survives the parent shell exiting.
- `/tmp/btcmap-server.pid` lets you `kill` it later.
- `/tmp/btcmap-server.log` captures startup logs and runtime errors.
- `ELECTRUM_URL` is **required** at runtime — if unset, `get_wallets` returns a JSON-RPC error (HTTP 200, `error.data: "ELECTRUM_URL env var must be set"`). The server still starts; other endpoints are unaffected.
- `ELECTRUM_INSECURE_TLS=1` (optional): disable TLS certificate validation for the Electrum connection. Accepts `1`/`true`/`yes`. Required for self-signed backends like `electrs.day.ag:50002`. A `WARN` log fires every RPC when set.

### Stop the server
```bash
kill "$(cat /tmp/btcmap-server.pid)"
sleep 1
ps -eo pid,cmd | awk '/target\/.*\/btcmap-api/ && !/awk/'
```
The second line is a sanity check — `ps` shouldn't show any `btcmap-api` process. If one is still there, `kill -9` it.

### Create a temporary admin token
Most admin RPCs (`get_wallets`, `dashboard`, etc.) require `Bearer` auth with a token that has the `admin` or `root` role. Don't reuse real production tokens. Mint a throwaway one against the local `main.db`:

```bash
sqlite3 $HOME/.local/share/btcmap/main.db <<'SQL'
INSERT INTO access_token (user_id, name, secret, roles)
VALUES (
  (SELECT id FROM user WHERE roles LIKE '%root%' LIMIT 1),
  'agent-probe',
  'probe-secret-CHANGE-ME-random-hex-here',
  '["root"]'
);
SQL
```
Pick a user with the `root` role (e.g. user id 3 if that's your local root account) and a `secret` string that's long and unique. The `secret` becomes the bearer token.

### Call an admin RPC
```bash
curl -sS \
  -X POST http://127.0.0.1:8000/rpc \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer probe-secret-CHANGE-ME-random-hex-here' \
  -d '{"jsonrpc":"2.0","id":"1","method":"get_wallets"}' \
  | python3 -m json.tool
```
- Endpoint: `POST /rpc` (not `/v2/rpc` or `/v3/rpc` — there's no version prefix).
- Method names are `snake_case` (e.g. `get_wallets`, `get_element`, `dashboard`).
- Response is JSON-RPC 2.0: `result` on success, `error` with `code`/`message`/`data` on failure.
- For slow calls, add `--max-time 180` to `curl`.

### Tear down the probe token
Always delete the throwaway token when you're done:
```bash
sqlite3 $HOME/.local/share/btcmap/main.db \
  "DELETE FROM access_token WHERE name='agent-probe';"
```
The token is in plain text in the DB, so leaving it around is the same as leaving a password in a config file. If you used a unique `name` (e.g. `agent-probe-1`, `agent-probe-2`), filter on that.

### Verifying changes against the real DB
Useful when you want to see if a patch actually works on production-shaped data (real element counts, real wallet history, etc.) rather than the empty in-memory test DB. The pattern is:
1. Make the code change.
2. `cargo build` (debug is faster).
3. Restart the server (kill + relaunch as above).
4. Hit the endpoint with `curl`, read the response.
5. Tail `/tmp/btcmap-server.log` for warnings/errors.
6. Iterate.

For wallet/sync RPCs especially, prefer this over adding unit tests — those endpoints talk to external services (Electrum, Overpass, OSM, LND) that aren't reachable from `cargo test`.

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
- Database stored in data directory (configurable via `data_dir_file_path`)
- Logging controlled via `RUST_LOG` env var (defaults to "info")
- `BTCMAP_API_BASE_URL` sets the public base URL of the API (used by the NIP-98 Nostr auth extractor). Defaults to `http://127.0.0.1:8000`.
- `BTCMAP_API_CORS_ORIGINS` controls the CORS middleware in `main.rs`. Unset or `*` (default) allows any origin. Set to a comma-separated list to restrict. The middleware handles the OPTIONS preflight itself.
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
