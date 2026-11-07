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

# Szybki check frontendu: SSR + hydrate (wasm32) + kompilacja testów (default features, jak w CI)
check-fe:
    cargo check -p kartoteka-frontend --features ssr
    cargo check -p kartoteka-frontend --features hydrate --target wasm32-unknown-unknown
    cargo nextest run -p kartoteka-frontend --no-run

build:
    cd crates/frontend && npm install
    cargo leptos build --release

# === DEPLOY ===

# Zbuduj obraz AMD64 lokalnie (Colima) i zdeployuj na preview
deploy-preview:
    bash scripts/deploy-preview.sh

# === QUALITY ===

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --check --all

fmt:
    cargo fmt --all

audit:
    cargo deny check

machete:
    cargo machete

# Testy — nextest dla testów, doctesty osobno (nextest je pomija)
test:
    cargo nextest run --workspace
    cargo test --workspace --doc

# Szybkie testy jednego crate'a (nextest), np. `just t domain -E 'test(/recurrence/)'`
t crate *args:
    cargo nextest run -p kartoteka-{{crate}} {{args}}

# Pojedyncze sprawdzenie przed pushem (kolejność: fmt → lint → test → check-fe)
verify:
    cargo fmt --check --all
    just lint
    just test
    just check-fe

# Tailwind 4 CSS — jednorazowa kompilacja do `crates/frontend/style/main.css` (plik jest
# generowany i gitignorowany, bez niego `cargo leptos build` nie wystartuje)
build-css:
    cd crates/frontend && npm install
    crates/frontend/node_modules/.bin/tailwindcss -i crates/frontend/style/input.css -o crates/frontend/style/main.css

# Uruchom testy e2e — CSS + cargo leptos build (SSR + WASM razem), serwer startuje Playwright
# (tests/playwright.config.ts: binarka z target/debug albo $KARTOTEKA_BIN, port 3030 albo $E2E_PORT)
test-e2e: build-css
    OAUTH_SIGNING_SECRET="${OAUTH_SIGNING_SECRET:-dev-secret-min-32-chars-abcdefgh}" cargo leptos build
    cd tests && npm install && CI=true npx playwright test --reporter=list

ci: fmt lint audit machete test
