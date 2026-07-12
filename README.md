# Motorsport data API (f1-api)

Public REST API for [f1.alchm.ist](https://f1.alchm.ist) and the wider alchm.ist motorsport platform.

Canonical host: `https://api.alchm.ist` (when DNS is ready). `https://api.f1.alchm.ist` continues to point at the same service for at least 12 months.

MIT licensed. Non-commercial, donation-supported community project.

## Disclaimer

f1.alchm.ist is an unofficial project and is not associated in any way with Formula 1 companies. F1, FORMULA ONE, FORMULA 1, FIA FORMULA ONE WORLD CHAMPIONSHIP, GRAND PRIX and related marks are trademarks of Formula One Licensing B.V.

Additional series marks (MotoGP, WRC, NASCAR, INDYCAR, Formula E, WEC, IMSA, GT World Challenge, and others) belong to their respective owners. This project is not affiliated with or endorsed by those rights holders.

## Non-commercial

This is a non-commercial, donation-supported community project. Supporter perks are a thank-you for keeping the servers running, not a product.

Support the servers: [Ko-fi](https://ko-fi.com/astroalchemist)

Legal contact: PLACEHOLDER_LEGAL_EMAIL

## Multi-series surface

```bash
curl -s http://127.0.0.1:8080/v1/series
curl -s http://127.0.0.1:8080/v1/motogp/events
curl -s http://127.0.0.1:8080/v1/f1/position
```

Legacy unscoped paths such as `/v1/position` remain as Formula 1 aliases. Prefer `/v1/{series}/...`.

Coverage differs by series. Some series are full/live, some are results-only. See `GET /v1/series` and `docs/guide.md`.

## Quick start

```bash
cp .env.example .env
cargo run
curl -s http://127.0.0.1:8080/v1/status
```

Requires `DATABASE_URL` (read-only Postgres role) and `REDIS_URL` for full operation. Without them the process still serves `/health` and `/v1/status`.

## Layout

| Path | Role |
| --- | --- |
| `crates/api-types` | Shared `/v1` and WebSocket contract types |
| `src/series` | Series registry |
| `src/policy` | Runtime distribution policy |
| `src/providers` | Public upstream result clients |
| `src/` | Axum service |
| `openapi.yaml` | OpenAPI 3.1 contract |
| `docs/guide.md` | Human guide |

## Bot identity

HTTP clients from this service identify as:

`alchmist-motorsport-bot/1.0 (+https://f1.alchm.ist/bot)`

## License

MIT. See `LICENSE`.
