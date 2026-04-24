# syntax=docker/dockerfile:1.7
ARG CARGO_PROFILE=release

FROM rustlang/rust:nightly-trixie AS builder
ARG CARGO_PROFILE

RUN apt-get update && apt-get install -y --no-install-recommends \
      curl build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# cargo-binstall installs prebuilt cargo-leptos binary (saves ~3 min vs from-source)
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash
RUN cargo binstall -y cargo-leptos

WORKDIR /app
COPY . .

# BuildKit cache mounts — persist crate registry, git checkouts, and target/
# between builds. `sharing=locked` prevents concurrent-build cache corruption.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
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
    && useradd -u 1001 -m app

COPY --from=builder /out-kartoteka /app/kartoteka
COPY --from=builder /out-site /app/site

WORKDIR /app
USER app

ENV BIND_ADDR=0.0.0.0:3000 \
    LEPTOS_SITE_ROOT=site \
    RUST_LOG=info

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["./kartoteka"]
