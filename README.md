# f1-api

Public REST and realtime API for [f1.alchm.ist](https://f1.alchm.ist).

MIT licensed. Non-commercial, donation-supported community project.

## Disclaimer

f1.alchm.ist is an unofficial project and is not associated in any way with Formula 1 companies. F1, FORMULA ONE, FORMULA 1, FIA FORMULA ONE WORLD CHAMPIONSHIP, GRAND PRIX and related marks are trademarks of Formula One Licensing B.V.

## Non-commercial

This is a non-commercial, donation-supported community project. Supporter perks are a thank-you for keeping the servers running, not a product.

Support the servers: [Buy Me a Coffee](PLACEHOLDER_BMAC_URL)

## Quick start

```bash
cp .env.example .env
cargo run
curl -s http://127.0.0.1:8080/v1/status
```

Requires `DATABASE_URL` (read-only Postgres role) and `REDIS_URL` for full operation. Without them the process still serves `/health` and a stub `/v1/status`.

## Layout

| Path | Role |
| --- | --- |
| `crates/api-types` | Shared `/v1` and WebSocket contract types |
| `src/` | Axum service |
| `openapi.yaml` | OpenAPI 3.1 contract |
| `docs/guide.md` | Human guide |

## License

MIT. See `LICENSE`.
