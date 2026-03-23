# Kartoteka

Aplikacja do zarządzania listami (zakupy, pakowanie, projekty, todo) z autoryzacją Hanko (passkeys + email).

## Stack

- **Frontend**: Leptos CSR (Rust → WASM), Trunk, PWA na iOS
- **Backend**: Cloudflare Workers (Rust, workers-rs), D1 (SQLite)
- **Auth**: Hanko Cloud (passkeys, email OTP)
- **MCP**: TypeScript CF Worker (scaffold, `@cloudflare/workers-oauth-provider`)

## Struktura

```
kartoteka/
├── crates/
│   ├── shared/       # Typy API (List, Item, DTOs)
│   ├── api/          # CF Worker — REST API + D1
│   └── frontend/     # Leptos CSR SPA + PWA
├── mcp/              # MCP Server (TypeScript, scaffold)
├── justfile          # Task runner
└── .env              # Konfiguracja (nie commitowana)
```

## Setup

```bash
# Wymagane: Rust, wasm32 target, Node.js, just
just setup

# Skopiuj i uzupełnij .env
cp .env.example .env
```

### `.env`

```
API_BASE_URL=https://your-api.workers.dev/api
CLOUDFLARE_ACCOUNT_ID=your-account-id
HANKO_API_URL=https://your-project.hanko.io
```

### D1

```bash
just db-create        # Utwórz bazę (jednorazowo)
just migrate-local    # Migracje lokalnie
just migrate-remote   # Migracje na produkcję
```

## Dev

```bash
just dev              # API + frontend równolegle
just dev-api          # Tylko API (localhost:8787)
just dev-frontend     # Tylko frontend (localhost:8080, proxy → API)
```

## Deploy

```bash
just deploy           # Wszystko: migracje + API + frontend + MCP
just deploy-api       # Tylko API worker
just deploy-frontend  # Tylko frontend (CF Pages)
```

## Komendy

| Komenda | Opis |
|---------|------|
| `just dev` | Dev lokalnie (API + frontend) |
| `just deploy` | Deploy na produkcję |
| `just check` | Sprawdź kompilację |
| `just lint` | Clippy + rustfmt |
| `just fmt` | Formatuj kod |
| `just test` | Testy |
| `just migrate-create NAME` | Nowa migracja D1 |
| `just migrate-local` | Migracje lokalnie |
| `just migrate-remote` | Migracje na produkcję |

## API

| Metoda | Endpoint | Opis |
|--------|----------|------|
| GET | `/api/health` | Health check |
| GET | `/api/lists` | Wszystkie listy |
| POST | `/api/lists` | Utwórz listę |
| GET | `/api/lists/:id` | Pobierz listę |
| PUT | `/api/lists/:id` | Aktualizuj listę |
| DELETE | `/api/lists/:id` | Usuń listę |
| GET | `/api/lists/:lid/items` | Items listy |
| POST | `/api/lists/:lid/items` | Dodaj item |
| PUT | `/api/lists/:lid/items/:id` | Aktualizuj item |
| DELETE | `/api/lists/:lid/items/:id` | Usuń item |

Endpointy (poza health) wymagają tokena Hanko w `Authorization: Bearer <token>`.

## Auth

Hanko Cloud — passkeys (Face ID / Touch ID) + email OTP jako fallback. Widget `<hanko-auth>` na stronie logowania, `<hanko-profile>` w ustawieniach.

Konfiguracja w Hanko Cloud dashboard:
- Authorized origins: `http://localhost:8080`, `https://your-frontend.pages.dev`
