# API guide

Non-commercial public API for timing and telemetry facts.

**Disclaimer.** f1.alchm.ist is an unofficial project and is not associated in any way with Formula 1 companies. F1, FORMULA ONE, FORMULA 1, FIA FORMULA ONE WORLD CHAMPIONSHIP, GRAND PRIX and related marks are trademarks of Formula One Licensing B.V.

**Non-commercial.** This is a non-commercial, donation-supported community project. Supporter perks are a thank-you for keeping the servers running, not a product.

## Base URL

`https://api.f1.alchm.ist/v1`

## Quick start

```bash
curl -s https://api.f1.alchm.ist/v1/status
```

API keys use the `Authorization: Bearer f1a_live_...` header (or `?key=`). Anonymous callers may use a small REST budget; live WebSocket access requires a key.

## What data is available and why

Timing and telemetry facts are distributed in full detail. Creative media is never rehosted.

| Class | Examples |
| --- | --- |
| Full detail | Laps, sectors, car data, positions, pit, tyres, race control text, weather, results, standings |
| Metadata only | Team radio timestamps and original `recording_url`; driver `headshot_url` as source link |
| Never | Logos, broadcast video/audio, onboard imagery, official app HTML/CSS assets |

Historical championship data is sourced via Jolpica with attribution.

## Tiers

| Tier | REST | Live stream |
| --- | --- | --- |
| anon | 1 rps / 30 per minute | none |
| free_key | 3 rps / 120 per minute | delayed (config) |
| supporter | 8 rps / 400 per minute | realtime |

Exact limits live in config and Redis and can change without a deploy.

## Fair use

Non-commercial use only. Resale is forbidden. Do not rehost audio or imagery. Attribution is appreciated.

## Status

This guide is the M0 scaffold. Endpoint reference and code samples land in later milestones. See `openapi.yaml` for the living contract.
