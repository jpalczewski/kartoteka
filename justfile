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

# Uruchom Gateway worker lokalnie
dev-gateway:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler dev --env local --local --port 8788

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

# === BUILD ===

# Sprawdź kompilację workspace
check:
    cargo check --workspace

# Build check for SSR server (fast, no WASM)
check-ssr:
    cargo check -p kartoteka-server -p kartoteka-frontend-v2 --features ssr

build: build-server build-gateway

build-server:
    cd crates/frontend-v2 && npm install
    cargo leptos build --release

build-gateway:
    cd gateway && npx wrangler deploy --dry-run

# === MIGRACJE ===

# Gateway auth DB migrations — uses /migrate endpoint (programmatic, Better Auth generates schema)
migrate-gateway-local:
    curl -X POST http://localhost:8788/migrate -H "x-migrate-secret: dev-migrate-secret"

migrate-gateway-dev:
    curl -X POST ${GATEWAY_DEV_URL}/migrate -H "x-migrate-secret: ${MIGRATE_SECRET_DEV}"

migrate-gateway-prod:
    curl -X POST https://kartoteka-gateway.jpalczewski.workers.dev/migrate -H "x-migrate-secret: ${MIGRATE_SECRET}"

# === DEPLOY ===

deploy: deploy-gateway migrate-gateway-prod

deploy-gateway-dev:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler deploy --env dev

deploy-dev: deploy-gateway-dev migrate-gateway-dev

deploy-gateway:
    cp -r locales gateway/locales
    cp locales/en/mcp.ftl gateway/locales/en/mcp.txt
    cp locales/pl/mcp.ftl gateway/locales/pl/mcp.txt
    cd gateway && npx wrangler deploy

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

# Uruchom testy e2e — najpierw cargo leptos build (SSR + WASM razem), potem Playwright
test-e2e:
    cargo leptos build
    cd tests && npm install && CI=true npm test

ci: fmt lint audit machete test
