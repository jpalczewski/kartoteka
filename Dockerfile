# syntax=docker/dockerfile:1.7
ARG CARGO_PROFILE=release

FROM node:22-slim AS css-builder
WORKDIR /app
COPY crates/frontend-v2/package.json crates/frontend-v2/
COPY crates/frontend-v2/style/ crates/frontend-v2/style/
COPY crates/frontend-v2/src/ crates/frontend-v2/src/
RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    cd crates/frontend-v2 && npm install && \
    npx @tailwindcss/cli -i style/input.css -o style/main.css --minify

FROM rustlang/rust:nightly-trixie AS builder
ARG CARGO_PROFILE

RUN apt-get update && apt-get install -y --no-install-recommends \
      curl build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# cargo-binstall installs prebuilt cargo-leptos binary (saves ~3 min vs from-source)
# Pinned to specific release to prevent supply-chain surprises from main-branch changes.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v1.18.1/install-from-binstall-release.sh \
    | bash
RUN cargo binstall -y cargo-leptos@0.3.5

WORKDIR /app
COPY . .
COPY --from=css-builder /app/crates/frontend-v2/style/main.css crates/frontend-v2/style/main.css

# BuildKit cache mounts — persist crate registry, git checkouts, and target/
# between builds. IDs match cache-map in workflow (buildkit-cache-dance).
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,id=cargo-git,sharing=locked \
    --mount=type=cache,target=/app/target,id=cargo-target,sharing=locked \
    set -eu; \
    if [ "$CARGO_PROFILE" = "release" ]; then \
        cargo leptos build --release; \
        cp target/release/kartoteka /out-kartoteka; \
    else \
        cargo leptos build; \
        cp target/debug/kartoteka /out-kartoteka; \
    fi; \
    cp -r target/site /out-site

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -u 1001 -m app \
    && mkdir -p /data && chown app:app /data

COPY --from=builder /out-kartoteka /app/kartoteka
COPY --from=builder /out-site /app/site
COPY locales/ /app/locales/
RUN chown -R app:app /app

WORKDIR /app
USER app

ENV BIND_ADDR=0.0.0.0:3000 \
    LEPTOS_SITE_ROOT=site \
    RUST_LOG=info \
    DATABASE_URL=sqlite:////data/data.db

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["./kartoteka"]
