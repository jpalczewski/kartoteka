# Plan 5: Deploy & Infrastructure — Design Spec

Parent: `docs/superpowers/specs/rewrite/2026-04-12-cloudflare-exit-v2-design.md`
Depends on: Plans 1-4

## Goal

Deploy the single Rust binary to Mikrus 4GB VPS. Caddy reverse proxy, systemd service, CI/CD, backups, background jobs, performance tuning, security hardening.

## Deploy

- **VPS:** Mikrus 4GB, existing Caddy reverse proxy
- **Binary:** `x86_64-unknown-linux-musl` (static linking)
- **Process:** systemd unit, `Restart=always`
- **SSL:** Caddy auto-SSL on custom domain
- **Backup:** cron `sqlite3 data.db '.backup ...'` daily
- **CI/CD:** GitHub Actions → cargo leptos build → scp + systemctl restart

### Env vars on VPS

```
DATABASE_URL=sqlite:///opt/kartoteka/data.db
PORT=3000
SESSION_SECRET=<random 64 bytes>
OAUTH_SIGNING_SECRET=<random 64 bytes>
BASE_URL=https://kartoteka.yourdomain.pl
RUST_LOG=kartoteka_server=info,kartoteka_domain=info,kartoteka_db=info,tower_http=info
```

### systemd unit

```ini
[Unit]
Description=Kartoteka
After=network.target

[Service]
ExecStart=/opt/kartoteka/kartoteka-server
WorkingDirectory=/opt/kartoteka
EnvironmentFile=/opt/kartoteka/.env
Restart=always

[Install]
WantedBy=multi-user.target
```

### Caddy config

```
kartoteka.yourdomain.pl {
    reverse_proxy localhost:3000

    @static path /pkg/*
    header @static Cache-Control "public, max-age=31536000, immutable"
}
```

### CI/CD (GitHub Actions)

```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/cache@v4
        with:
          path: target
          key: cargo-${{ hashFiles('Cargo.lock') }}

      - name: Build
        run: |
          cargo leptos build --release --precompress

      - name: Deploy
        run: |
          scp target/server/release/kartoteka-server user@$VPS:/opt/kartoteka/kartoteka-server-new
          scp -r target/site/* user@$VPS:/opt/kartoteka/site/
          ssh user@$VPS 'mv /opt/kartoteka/kartoteka-server-new /opt/kartoteka/kartoteka-server && systemctl restart kartoteka'
        env:
          VPS: ${{ secrets.VPS_HOST }}
```

### Backup

```bash
# /etc/cron.d/kartoteka-backup
0 4 * * * root sqlite3 /opt/kartoteka/data.db '.backup /opt/kartoteka/backups/data-$(date +\%F).db' && find /opt/kartoteka/backups -mtime +30 -delete
```

## Performance

### HTTP compression

`tower_http::CompressionLayer` on the Axum router — automatic gzip/brotli on responses (HTML, JSON, CSS). Single layer, handles Accept-Encoding negotiation.

### Static file caching

cargo-leptos config:
```toml
hash-files = true                               # content-hashed filenames → immutable caching
wasm-opt-features = ["-Oz", "--enable-bulk-memory"]  # WASM size optimization
```

Build: `cargo leptos build --release --precompress` — pre-generates .gz + .br files.

Hashed filenames mean infinite cache — browser never re-downloads unchanged assets. Caddy `@static` rule sets `Cache-Control: immutable`.

### SQLite tuning

See db-domain spec for full details. Key pragmas: WAL, mmap_size=256MB, busy_timeout=5000, synchronous=NORMAL.

## Background Jobs (apalis)

`apalis` + `apalis-sqlite` + `apalis-cron` — persistent job queue backed by SQLite. Tower-based middleware (retry, rate-limit, tracing).

### Job types

| Job | Schedule | Purpose |
|-----|----------|---------|
| `CleanupSessionsJob` | cron `0 * * * *` (hourly) | Delete expired sessions |
| `CleanupOAuthCodesJob` | cron `0 * * * *` (hourly) | Delete expired auth codes + refresh tokens |
| `MaintenanceJob` | cron `0 3 * * 0` (weekly Sun 3am) | SQLite VACUUM + ANALYZE |
| `OptimizeJob` | cron `0 */6 * * *` (every 6h) | PRAGMA optimize |
| `SendNotificationJob` | one-off, delayed | Send via Telegram/web push (retry with backoff) |

### Architecture

```
crates/server/src/
  jobs/
    mod.rs              — apalis worker setup, register all job types
    maintenance.rs      — VacuumJob, AnalyzeJob, CleanupExpiredJob
    notifications.rs    — SendNotificationJob (one-off, retry)
    channels/
      telegram.rs       — Telegram Bot API call
```

Jobs persist in SQLite — survive restart. Domain layer can enqueue jobs:

```rust
storage.push(SendNotificationJob {
    user_id, channel: "telegram",
    payload: "Deadline za 1h: Kup mleko",
}).await?;
```

### Future job types (not implemented now)

- Deadline reminder notifications (scan items approaching deadline, enqueue SendNotificationJob)
- iCal feed cache regeneration (#31 — after item date mutations, regenerate .ics)
- Real-time SSE event dispatch (#30)

## Security hardening

- Rate limiting on `/auth/login`, `/auth/register`, `/oauth/register`, `/oauth/token` (tower-governor or custom middleware)
- CSRF token on OAuth consent form POST
- OAuth `state` parameter for authorization endpoint CSRF protection
- DCR abuse prevention (limit clients per IP or require auth)
- Refresh token rotation (new refresh token on each use, invalidate old)
- Token scope enforcement in MCP tools
- CORS policy configuration (allowed origins for REST API)
- Expired OAuth authorization codes cleanup (background task like session cleanup)

## Relation to existing issues

### iCal feed (#31, #32, #34)

The iCal feed (`GET /cal/{token}/feed.ics`) is a natural extension of this architecture:

- **Auth:** `calendar_tokens` table with crypto-random bearer tokens — extends the bearer token auth pattern from Plan 2. Token in URL determines user + scope (single list or all lists).
- **Domain:** `domain::calendar::generate_ical(pool, user_id, scope, reminder_settings)` — uses `icalendar` crate, calls `domain::items::by_date` for data. Reminder defaults (VALARM) from user settings (#31), per-list/per-item overrides (#32) from list_features/item fields.
- **Background jobs:** `RegenerateIcalCacheJob` — enqueued by domain:: after item mutations with date fields.
- **Recurring items (#34):** RRULE generation in iCal events. Depends on recurrence model in domain.

### Real-time updates (#30)

SSE (Server-Sent Events) after VPS migration:
- **Domain:** After mutations, domain:: publishes to `tokio::sync::broadcast` channel.
- **Axum:** SSE endpoint (`GET /api/events`) with `AuthSession`, streams events.
- **Frontend:** `EventSource` (behind `#[cfg(feature = "hydrate")]`) subscribes, triggers Resource refetch.
