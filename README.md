# yoobu-notifier

Rust service for sending Telegram notifications. Listens to the `notification_outbox` table in PostgreSQL via `LISTEN/NOTIFY` and delivers messages through the Telegram Bot API.

Part of the [yoobu](https://github.com/solairerove/yoobu-api) stack. Replaces the in-process Spring-based notification mechanism.

## How it works

```
yoobu-api (Java)
  └── INSERT INTO notification_outbox + NOTIFY notification_outbox
        └── yoobu-notifier (Rust)
              └── LISTEN → read outbox → Telegram Bot API
```

The bot token is fetched from the `tenant` table by `tenant_id` from the payload. Each tenant uses their own bot.

## Requirements

- Rust 1.75+
- PostgreSQL 15+ (same instance as `yoobu-api`)
- Migration `V14__notification_outbox.sql` from `yoobu-api` applied
- Running `yoobu-api` (writes events to the outbox)

## Environment variables

| Variable | Description | Example |
|---|---|---|
| `DB_URL` | JDBC-style connection URL | `jdbc:postgresql://localhost:5432/yoobu` |
| `DB_USER` | DB user | `yoobu` |
| `DB_PASS` | DB password | `secret` |
| `RUST_LOG` | Log level | `yoobu_notifier=info` |

Copy `.env.example` to `.env` and fill in the values:

```bash
cp .env.example .env
```

## Running locally

Make sure PostgreSQL is running (via `yoobu-api`):

```bash
# from yoobu-api/
docker-compose up -d postgres
```

Start the service:

```bash
cargo run
```

Or build and run the binary:

```bash
cargo build --release
./target/release/yoobu-notifier
```

## Running with Docker Compose

Requires the `yoobu-net` network and `yoobu-postgres` container to be running:

```bash
# create the network if it doesn't exist yet
docker network create yoobu-net

# from yoobu-api/ — start the database
docker-compose up -d postgres

# from yoobu-notifier/
docker-compose up -d
```

Logs:

```bash
docker-compose logs -f yoobu-notifier
```

## Building the Docker image

```bash
docker build -t yoobu-notifier .
```

## Deploying to Railway

The service is deployed as a separate Railway service within the same project as `yoobu-api`.

The service shares variables with `yoobu-api` at the project level:

| Variable | Value |
|---|---|
| `DB_URL` | `jdbc:postgresql://postgres.railway.internal:5432/railway` |
| `DB_USER` | `${{Postgres.PGUSER}}` |
| `DB_PASS` | `${{Postgres.PGPASSWORD}}` |
| `RUST_LOG` | `yoobu_notifier=info` |
