# Kartoteka

A list management app (shopping, packing, projects, todos) with Hanko authentication (passkeys + email).

## Features

- **Lists** — create, delete (with confirmation modal showing item count), types: list / shopping / packing / project
- **Items** — add, toggle completed, delete, edit description, optimistic updates
- **Tags** — manage tags (color), assign to lists and items, filter lists by tag
- **Notifications** — global toast system (success / error), auto-dismiss after 3s
- **Auth** — Hanko Cloud: passkeys (Face ID / Touch ID) + email OTP

## Stack

- **Frontend**: Leptos CSR (Rust → WASM), Trunk, PWA on iOS
- **Backend**: Cloudflare Workers (Rust, workers-rs), D1 (SQLite)
- **Auth**: Hanko Cloud (passkeys, email OTP)
- **MCP**: TypeScript CF Worker (scaffold, `@cloudflare/workers-oauth-provider`)

## Structure

```
kartoteka/
├── crates/
│   ├── shared/       # API types (List, Item, DTOs)
│   ├── api/          # CF Worker — REST API + D1
│   └── frontend/     # Leptos CSR SPA + PWA
├── mcp/              # MCP Server (TypeScript, scaffold)
├── justfile          # Task runner
└── .env              # Config (not committed)
```

## Setup

```bash
# Required: Rust, wasm32 target, Node.js, just
just setup

# Copy and fill in .env
cp .env.example .env
```

### `.env`

```
API_BASE_URL=https://your-api.workers.dev/api
CLOUDFLARE_ACCOUNT_ID=your-account-id
HANKO_API_URL=https://your-project.hanko.io
```

### D1

The project uses three environments with separate D1 databases:

| Environment | Wrangler env | D1 database |
|-------------|-------------|-------------|
| local | `local` | SQLite locally (Miniflare) |
| dev | `dev` | `kartoteka-dev` (CF D1) |
| prod | *(default)* | `kartoteka` (CF D1) |

```bash
just db-create        # Create prod database (once)
just db-create-dev    # Create dev database (once)
just migrate-local    # Run migrations locally
just migrate-remote   # Run migrations on production
```

## Dev

```bash
just dev              # API + frontend in parallel
just dev-api          # API only (localhost:8787)
just dev-frontend     # Frontend only (localhost:8080, proxy → API)
```

## Deploy

```bash
just deploy           # Everything: migrations + API + frontend + MCP
just deploy-api       # API worker only
just deploy-frontend  # Frontend only (CF Pages)
```

## Commands

| Command | Description |
|---------|-------------|
| `just dev` | Local dev (API + frontend) |
| `just deploy` | Deploy to production |
| `just check` | Check compilation |
| `just lint` | Clippy + rustfmt |
| `just fmt` | Format code |
| `just migrate-create NAME` | New D1 migration |
| `just migrate-local` | Run migrations locally |
| `just migrate-remote` | Run migrations on production |

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check |
| GET | `/api/lists` | All lists |
| POST | `/api/lists` | Create list |
| GET | `/api/lists/:id` | Get list |
| PUT | `/api/lists/:id` | Update list |
| DELETE | `/api/lists/:id` | Delete list (cascades to items and tags) |
| GET | `/api/lists/:lid/items` | List items |
| POST | `/api/lists/:lid/items` | Add item |
| PUT | `/api/lists/:lid/items/:id` | Update item |
| DELETE | `/api/lists/:lid/items/:id` | Delete item |
| GET | `/api/tags` | All tags |
| POST | `/api/tags` | Create tag |
| PUT | `/api/tags/:id` | Update tag |
| DELETE | `/api/tags/:id` | Delete tag |
| GET | `/api/list-tags` | List–tag links |
| POST | `/api/lists/:lid/tags/:tid` | Assign tag to list |
| DELETE | `/api/lists/:lid/tags/:tid` | Remove tag from list |
| GET | `/api/item-tags` | Item–tag links |
| POST | `/api/items/:iid/tags/:tid` | Assign tag to item |
| DELETE | `/api/items/:iid/tags/:tid` | Remove tag from item |

All endpoints (except health) require a Hanko token in `Authorization: Bearer <token>`.

## Auth

Hanko Cloud — passkeys (Face ID / Touch ID) + email OTP as fallback. The `<hanko-auth>` widget on the login page, `<hanko-profile>` in settings.

Hanko Cloud dashboard configuration:
- Authorized origins: `http://localhost:8080`, `https://your-frontend.pages.dev`

## Trivia

### The Great Cloudflare Workers CPU Crisis

The MCP server uses OAuth 2.1 with PKCE for Claude Code authentication. On Cloudflare Workers,
`@cloudflare/workers-oauth-provider` performs 6 `crypto.subtle` operations per OAuth flow
(AES-256-GCM key generation, encryption, HMAC, AES-KW key wrapping) to encrypt token props.
Combined with Better Auth's pure-JS scrypt password hashing (~2000ms CPU), this comfortably
exceeds the free tier's 10ms CPU limit — and even struggles on the paid tier.

The irony: Cloudflare's own OAuth library doesn't fit on Cloudflare's own free plan.

This led to the decision to migrate off Cloudflare entirely to a single Rust binary on a
[Mikrus Frog](https://mikr.us/) VPS (256MB RAM, 3GB disk, free forever for 5 PLN activation).
See `docs/superpowers/specs/2026-03-28-cloudflare-exit-rewrite-design.md` for the full design.
