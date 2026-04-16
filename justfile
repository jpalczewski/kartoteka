# Kartoteka — task runner
set dotenv-load

export CLOUDFLARE_ACCOUNT_ID := env("CLOUDFLARE_ACCOUNT_ID", "")

default:
    @just --list

# === SETUP ===

# Zainstaluj wymagane narzędzia
setup:
    cargo install cargo-leptos
    rustup target add wasm32-unknown-unknown
    cd crates/frontend-v2 && npm install

# === DEV ===

# Uruchom API worker lokalnie
dev-api:
    cd crates/api && npx wrangler dev --env local --local --port 8787

# Uruchom Gateway worker lokalnie
dev-gateway:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler dev --env local --local --port 8788

# Uruchom frontend (proxy config in Trunk.toml)
dev-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && trunk serve

# Wystaw Gateway przez HTTPS via cloudflared tunnel
dev-tunnel:
    cloudflared tunnel --url http://localhost:8788

# Uruchom SSR server + Tailwind watch (nowy rewrite)
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    just dev-tailwind &
    just dev-leptos &
    wait

# Tailwind 4 CSS compilation (watch mode)
dev-tailwind:
    crates/frontend-v2/node_modules/.bin/tailwindcss -i crates/frontend-v2/style/input.css -o crates/frontend-v2/style/main.css --watch

# SSR server: cargo-leptos hot reload
dev-leptos:
    OAUTH_SIGNING_SECRET="${OAUTH_SIGNING_SECRET:-dev-secret-min-32-chars-abcdefgh}" cargo leptos watch

# [legacy] Stary CF Workers dev (deprecated)
dev-ssr:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    just dev-tailwind &
    just dev-leptos &
    wait

# [legacy] Stary CF Workers stack
dev-cf:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    just dev-api &
    just dev-gateway &
    just dev-frontend &
    just dev-tunnel &
    wait

# === BUILD ===

# Sprawdź kompilację workspace
check:
    cargo check --workspace

# Build check for SSR server (fast, no WASM)
check-ssr:
    cargo check -p kartoteka-server -p kartoteka-frontend-v2 --features ssr

build: build-api build-frontend build-gateway

build-api:
    cd crates/api && worker-build --release

build-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && trunk build --release

build-gateway:
    cd gateway && npx wrangler deploy --dry-run

# === MIGRACJE ===

# Utwórz nową migrację D1 (API worker)
migrate-create NAME:
    cd crates/api && npx wrangler d1 migrations create kartoteka-db {{NAME}}

# Zastosuj migracje API lokalnie
migrate-local:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-api-local --env local --local

# Zastosuj migracje API na dev
migrate-dev:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-dev --env dev --remote

# Zastosuj migracje API na produkcję
migrate-prod:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --env="" --remote

migrate-remote: migrate-prod

# Gateway auth DB migrations — uses /migrate endpoint (programmatic, Better Auth generates schema)
migrate-gateway-local:
    curl -X POST http://localhost:8788/migrate -H "x-migrate-secret: dev-migrate-secret"

migrate-gateway-dev:
    curl -X POST ${GATEWAY_DEV_URL}/migrate -H "x-migrate-secret: ${MIGRATE_SECRET_DEV}"

migrate-gateway-prod:
    curl -X POST https://kartoteka-gateway.jpalczewski.workers.dev/migrate -H "x-migrate-secret: ${MIGRATE_SECRET}"

# === DEPLOY ===

deploy: deploy-migrate deploy-api deploy-gateway migrate-gateway-prod deploy-frontend

deploy-gateway-dev:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler deploy --env dev

deploy-dev: migrate-dev deploy-api-dev deploy-gateway-dev migrate-gateway-dev deploy-frontend-dev

deploy-migrate:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --remote

deploy-api:
    cd crates/api && npx wrangler deploy

deploy-api-dev:
    cd crates/api && npx wrangler deploy --env dev

deploy-gateway:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler deploy

deploy-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${GATEWAY_URL}/api" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=main --commit-dirty=true

deploy-frontend-dev:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${GATEWAY_DEV_URL}/api" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=dev --commit-dirty=true

# === QUALITY ===

lint:
    cargo clippy --workspace -- -D warnings
    cargo fmt --check --all

fmt:
    cargo fmt --all

audit:
    cargo deny check

machete:
    cargo machete

test:
    cargo test --workspace

# Uruchom testy e2e (wymaga działającego just dev)
test-e2e:
    cd tests && npm install && npx playwright test --reporter=list

ci: fmt lint audit machete test
