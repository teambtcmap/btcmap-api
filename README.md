# BTC Map API

### Check our [API Docs](https://github.com/teambtcmap/btcmap-api/blob/master/docs%2FREADME.md) for more information.

## Local Development

### Prerequisites

1. **Install Rust** via [rustup](https://rust-lang.org/tools/install/):

   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   The project pins its Rust version in `rust-toolchain.toml` — rustup will automatically install and use the correct version.

2. **Create the data directory** for the SQLite databases:

   ```
   mkdir -p ~/.local/share/btcmap
   ```

3. **Install sqlite3_rsync** (needed to fetch the production database):

   ```
   # macOS
   brew install sqlite-rsync
   ```

4. **Configure SSH access** to the production server (needed for fetching data). Add to `~/.ssh/config`:

   ```
   Host btcmap-api
     User root
     Hostname <server-ip>
   ```

### Build

```
cargo build
```

### Test

```
cargo test
```

### Fetch Production Data

The API needs a database to serve data. Fetch a copy of the production database:

```
./devtools fetch-main-db
```

This uses `sqlite3_rsync` to sync the main database from the production server.

### Run

```
cargo run
```

The server binds to `http://127.0.0.1:8000`. Test it with:

```
curl http://localhost:8000/v2/areas
```

### Configuration

Behavior is controlled by environment variables (all optional in local dev):

| Variable | Default | Purpose |
|----------|---------|---------|
| `RUST_LOG` | `info` | Log level. |
| `BTCMAP_API_BASE_URL` | `http://127.0.0.1:8000` | Public base URL of the API. NIP-98 Nostr auth verifies the signed event's `u` tag against this value, **not** the request `Host`/`X-Forwarded-*` headers. **In production this must be set to the public origin** (e.g. `https://api.btcmap.org`) or all Nostr auth fails with `401`. See [Server Configuration (NIP-98)](docs/rest/v4/auth.md#server-configuration-nip-98). |

The CORS allowlist lives in the `conf` table (column `cors_origins`): a
comma-separated list of allowed origins. Empty (the default) allows any
origin. Update the row directly, e.g.:

```sql
UPDATE conf SET cors_origins = 'https://btcmap.org,https://dashboard.btcmap.org';
```

### devtools

The `devtools` script provides helper commands for development:

| Command | Description |
|---------|-------------|
| `main-db [query]` | Open the main database in sqlite3 (or run a query) |
| `image-db [query]` | Open the image database in sqlite3 |
| `log-db [query]` | Open the log database in sqlite3 |
| `fetch-db` | Fetch all databases from production |
| `fetch-main-db` | Fetch only the main database |
| `fetch-image-db` | Fetch only the image database |
| `fetch-log-db` | Fetch only the log database |
| `deploy` | Run tests, build release, deploy to production |
| `gen-main-schema` | Generate `schema.sql` from migrations |
| `export-ts-types [dir]` | Export the TypeScript bindings to a directory |

### TypeScript bindings (`bindings/ts/`)

`bindings/ts/` contains TypeScript definitions for the v4 REST types that
[btcmap.org](https://github.com/teambtcmap/btcmap.org) consumes. They are
**generated, not hand-written**: structs annotated with `#[derive(ts_rs::TS)]`
are exported by [ts-rs](https://github.com/Aleph-Alpha/ts-rs) every time
`cargo test` runs, so the directory always matches the code on your branch and
never goes stale silently — CI fails if a commit leaves it out of date.

How the pieces fit:

- **Changing an exported struct?** Run `cargo test` (or
  `devtools export-ts-types`), and commit the updated `bindings/ts/` files along
  with your change. The diff doubles as a readable record of the API change.
- **Adding a new response type for the frontend?** Add
  `#[derive(ts_rs::TS)]` + `#[ts(export)]` to the struct. Conventions:
  64-bit integers get `#[ts(type = "number")]` (JSON transport), RFC 3339
  timestamps get `#[ts(type = "string")]`, and names that repeat across
  modules get `#[ts(rename = "...")]` so every binding file is unique.
  Export is opt-in per struct — types not meant for third-party use simply
  don't get the derive.
- **Other languages:** the `ts/` subdirectory leaves room for bindings in
  other languages (Kotlin, Swift, ...) to live alongside it under `bindings/`.
- **Consuming the types?** The frontend fetches this directory from GitHub
  (`pnpm types:api` in btcmap.org) — no Rust toolchain or checkout of this
  repo required. Any other client can do the same.

The dynamic `GET /v4/places` responses (shaped by the `fields` query param)
are described by the all-optional `Place` type, kept in sync with
`service::element::TAGS` by the `place_type_covers_all_generate_tags_fields`
test.
| `install-completions` | Install bash tab completions for devtools |
