# Message

Rust/Axum API for contact-form topics with optional Telegram forwarding.

## Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/ping` | No | Health check (returns `pong`) |
| `GET` | `/topics` | Yes | List topics (returns `id`, `name`) |
| `POST` | `/topics` | Yes | Create topic (optional `tg_api`) |
| `POST` | `/topics/:topic_id/messages` | No | Submit a message (public) |
| `GET` | `/topics/:topic_id/messages` | Yes | List messages for a topic |

### Create topic body

```json
{
  "name": "Contact form",
  "tg_api": {
    "api_key": "123456:ABC-...",
    "chat_id": "-100..."
  }
}
```

`tg_api` is optional. When provided, the bot token is validated via Telegram's `getMe` before the topic is created.

### Submit message body

```json
{
  "contacts": { "email": "user@example.com" },
  "text": "Hello!"
}
```

`contacts` is freeform JSON.

## Auth

Bearer token via `Authorization` header, checked against the `AUTH_TOKEN` env var. If `AUTH_TOKEN` is not set, all endpoints are open.

## Telegram integration

Topics can have a `tg_api` config (`api_key` + `chat_id`). When a message is submitted to such a topic, it is forwarded asynchronously to the configured Telegram chat.

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `POSTGRES_URL` | Yes | Postgres connection string |
| `HOST` | Yes | Bind address |
| `PORT` | Yes | Bind port |
| `AUTH_TOKEN` | No | Bearer token for protected endpoints |
| `RUST_LOG` | No | Log level filter (e.g. `info`, `debug`) |

## Running

```bash
cargo run
```

Docker:

```bash
docker run -e POSTGRES_URL=... -e HOST=0.0.0.0 -e PORT=3000 ghcr.io/ku113p/message:latest
```

Database migrations run automatically on startup.
