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
| `BTCMAP_API_CORS_ORIGINS` | `*` | Comma-separated allowlist of CORS origins. `*` (or unset) allows any origin. |

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
| `install-completions` | Install bash tab completions for devtools |
