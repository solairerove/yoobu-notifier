# ── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM rust:slim-bookworm AS builder

WORKDIR /app

COPY . .
RUN cargo build --release

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/yoobu-notifier /usr/local/bin/yoobu-notifier

CMD ["yoobu-notifier"]
