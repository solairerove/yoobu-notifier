# syntax=docker/dockerfile:1

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM rust:slim-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
    && cp target/release/yoobu-notifier /tmp/yoobu-notifier

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/yoobu-notifier /usr/local/bin/yoobu-notifier

CMD ["yoobu-notifier"]
