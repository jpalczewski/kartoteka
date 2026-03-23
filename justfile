# Kartoteka — task runner
set dotenv-load

export CLOUDFLARE_ACCOUNT_ID := env("CLOUDFLARE_ACCOUNT_ID", "")

# Generuje hanko-init.js z template
_gen-hanko:
    sed 's|__HANKO_API_URL__|'"${HANKO_API_URL}"'|g' crates/frontend/hanko-init.js.template > crates/frontend/hanko-init.js

default:
    @just --list

# === SETUP ===

# Zainstaluj wymagane narzędzia
setup:
    cargo install trunk worker-build
    rustup target add wasm32-unknown-unknown
    cd mcp && npm install
    cd crates/frontend && npm install

# Utwórz bazę D1 (jednorazowo)
db-create:
    cd crates/api && npx wrangler d1 create kartoteka-db

# === DEV ===

# Uruchom API worker lokalnie
dev-api:
    cd crates/api && HANKO_API_URL="${HANKO_API_URL}" npx wrangler dev

# Uruchom frontend z proxy do API
dev-frontend: _gen-hanko
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="/api" HANKO_API_URL="${HANKO_API_URL}" trunk serve --proxy-backend=http://127.0.0.1:8787/api

# Uruchom MCP server lokalnie
dev-mcp:
    cd mcp && npx wrangler dev

# Uruchom API + frontend równolegle
dev:
    just dev-api & just dev-frontend & wait

# === BUILD ===

# Zbuduj wszystko
build: build-api build-frontend build-mcp

# Zbuduj API worker
build-api:
    cd crates/api && worker-build --release

# Zbuduj frontend
build-frontend: _gen-hanko
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${API_BASE_URL}" HANKO_API_URL="${HANKO_API_URL}" trunk build --release

# Zbuduj MCP server
build-mcp:
    cd mcp && npx wrangler deploy --dry-run

# Sprawdź kompilację workspace
check:
    API_BASE_URL="/api" HANKO_API_URL="${HANKO_API_URL}" cargo check --workspace

# === MIGRACJE ===

# Utwórz nową migrację D1
migrate-create NAME:
    cd crates/api && npx wrangler d1 migrations create kartoteka-db {{NAME}}

# Zastosuj migracje lokalnie
migrate-local:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --local

# Zastosuj migracje na produkcję
migrate-remote:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --remote

# === DEPLOY ===

# Deploy wszystkiego na produkcję
deploy: deploy-migrate deploy-api deploy-frontend deploy-mcp

# Deploy migracji
deploy-migrate:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --remote

# Deploy API worker
deploy-api:
    cd crates/api && HANKO_API_URL="${HANKO_API_URL}" npx wrangler deploy

# Deploy frontend na CF Pages
deploy-frontend: build-frontend
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=main --commit-dirty=true

# Deploy MCP server
deploy-mcp:
    cd mcp && npx wrangler deploy

# === QUALITY ===

# Lint + format check
lint:
    API_BASE_URL="/api" HANKO_API_URL="${HANKO_API_URL}" cargo clippy --workspace -- -D warnings
    cargo fmt --check --all

# Format kodu
fmt:
    cargo fmt --all

# Uruchom testy
test:
    API_BASE_URL="/api" HANKO_API_URL="${HANKO_API_URL}" cargo test --workspace
