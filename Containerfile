# syntax=docker/dockerfile:1

# ─── Stage 1: Builder ──────────────────────────────────────────────────────
FROM rust:1-alpine AS builder

RUN apk add --no-cache \
    bash curl \
    nodejs npm \
    binaryen \
    musl-dev clang

# cargo-leptos — warstwa zakeszowana dopóki ten RUN się nie zmieni
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-binstall --locked && \
    cargo binstall cargo-leptos --locked -y

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# npm deps — zakeszowane osobno (zmienia się tylko przy package-lock.json)
COPY crates/frontend/package.json crates/frontend/package-lock.json \
     crates/frontend/
RUN --mount=type=cache,target=/root/.npm \
    npm ci --prefix crates/frontend

# Kod źródłowy
COPY . .

# Build z cache na registry + target (kompilacja przyrostowa między buildami)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo leptos build --release && \
    cp target/release/kartoteka /app/kartoteka_bin && \
    cp -r target/site /app/site_build

# ─── Stage 2: Runtime (~20 MB) ─────────────────────────────────────────────
FROM alpine:3.21 AS runtime
WORKDIR /app

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/kartoteka_bin /app/kartoteka
COPY --from=builder /app/site_build    /app/site

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ROOT="/app/site"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"

EXPOSE 8080
CMD ["/app/kartoteka"]
