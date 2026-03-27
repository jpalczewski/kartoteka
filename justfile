# Kartoteka — task runner
set dotenv-load

export CLOUDFLARE_ACCOUNT_ID := env("CLOUDFLARE_ACCOUNT_ID", "")

default:
    @just --list

# === SETUP ===

# Zainstaluj wymagane narzędzia
setup:
    cargo install trunk worker-build
    rustup target add wasm32-unknown-unknown
    cd gateway && npm install
    cd crates/frontend && npm install

# === DEV ===

# Uruchom API worker lokalnie
dev-api:
    cd crates/api && npx wrangler dev --env local --local --port 8787

# Uruchom Gateway worker lokalnie
dev-gateway:
    cd gateway && npx wrangler dev --env local --local --port 8788

# Uruchom frontend (proxy config in Trunk.toml)
dev-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="/api" trunk serve

# Uruchom API + Gateway + frontend
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    just dev-api &
    just dev-gateway &
    just dev-frontend &
    wait

# === BUILD ===

# Sprawdź kompilację workspace
check:
    API_BASE_URL="/api" cargo check --workspace

build: build-api build-frontend build-gateway

build-api:
    cd crates/api && worker-build --release

build-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${API_BASE_URL}" trunk build --release

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

migrate-gateway-prod:
    curl -X POST https://kartoteka-gateway.jpalczewski.workers.dev/migrate -H "x-migrate-secret: ${MIGRATE_SECRET}"

# === DEPLOY ===

deploy: deploy-migrate migrate-gateway-prod deploy-api deploy-gateway deploy-frontend

deploy-dev: migrate-dev deploy-api-dev deploy-frontend-dev

deploy-migrate:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --remote

deploy-api:
    cd crates/api && npx wrangler deploy

deploy-api-dev:
    cd crates/api && npx wrangler deploy --env dev

deploy-gateway:
    cd gateway && npx wrangler deploy

deploy-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${API_BASE_URL}" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=main --commit-dirty=true

deploy-frontend-dev:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="https://kartoteka-gateway.jpalczewski.workers.dev/api" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=dev --commit-dirty=true

# === QUALITY ===

lint:
    API_BASE_URL="/api" cargo clippy --workspace -- -D warnings
    cargo fmt --check --all

fmt:
    cargo fmt --all

audit:
    cargo deny check

machete:
    cargo machete

test:
    API_BASE_URL="/api" cargo test --workspace

# Uruchom testy e2e (wymaga działającego just dev)
test-e2e:
    cd tests && npm install && npx playwright test --reporter=list

ci: fmt lint audit machete test
