# Kartoteka — task runner
set dotenv-load

default:
    @just --list

# === SETUP ===

# Zainstaluj wymagane narzędzia
setup:
    cargo install cargo-leptos
    rustup target add wasm32-unknown-unknown
    cd crates/frontend && npm install

# === DEV ===

# Uruchom SSR server + Tailwind watch (nowy rewrite)
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    just dev-tailwind &
    just dev-leptos &
    wait

# Tailwind 4 CSS compilation (watch mode)
dev-tailwind:
    crates/frontend/node_modules/.bin/tailwindcss -i crates/frontend/style/input.css -o crates/frontend/style/main.css --watch

# SSR server: cargo-leptos hot reload
dev-leptos:
    OAUTH_SIGNING_SECRET="${OAUTH_SIGNING_SECRET:-dev-secret-min-32-chars-abcdefgh}" cargo leptos watch

# === BUILD ===

# Sprawdź kompilację workspace
check:
    cargo check --workspace

# Build check for SSR server (fast, no WASM)
check-ssr:
    cargo check -p kartoteka-server -p kartoteka-frontend --features ssr

build:
    cd crates/frontend && npm install
    cargo leptos build --release

# === DEPLOY ===

# Zbuduj obraz AMD64 lokalnie (Colima) i zdeployuj na preview
deploy-preview:
    bash scripts/deploy-preview.sh

# Pobierz logi z Coolify. Bez argumentów: lista aplikacji z UUID-ami.
# Z UUID: just logs <uuid> [liczba-linii]
logs *args:
    bash scripts/coolify-logs.sh {{args}}

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
