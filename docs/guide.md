# API guide

Non-commercial public API for motorsport timing and result facts across multiple series.

**Disclaimer.** f1.alchm.ist is an unofficial project and is not associated in any way with Formula 1 companies. F1, FORMULA ONE, FORMULA 1, FIA FORMULA ONE WORLD CHAMPIONSHIP, GRAND PRIX and related marks are trademarks of Formula One Licensing B.V. Other series marks belong to their respective owners; this project is not affiliated with or endorsed by those rights holders.

**Non-commercial.** This is a non-commercial, donation-supported community project. Supporter perks are a thank-you for keeping the servers running, not a product.

Legal contact: PLACEHOLDER_LEGAL_EMAIL

## Base URL

Canonical: `https://api.alchm.ist/v1` (when live)

Compatibility: `https://api.f1.alchm.ist/v1` (same service, supported for at least 12 months)

## Series registry

```bash
curl -s https://api.f1.alchm.ist/v1/series
```

Each series exposes `coverage` (`full`, `live`, or `results-only`), `enabled_endpoints`, and a `disclaimer_key`. Prefer scoped paths:

```bash
curl -s https://api.f1.alchm.ist/v1/motogp/events
curl -s https://api.f1.alchm.ist/v1/f1/position
curl -s https://api.f1.alchm.ist/v1/wrc/events
```

Unscoped legacy paths such as `/v1/position` remain Formula 1 aliases.

## Coverage and why it differs per series

The community promise is honesty, not identical depth for every series.

| Coverage | Meaning |
| --- | --- |
| full | Live and historical timing facts where a public upstream exists |
| live | Calendar and results are available; live may be limited or gated |
| results-only | Final classification facts only. No live timing, no lap/sector detail from proprietary timing vendors |

Results-only series exist because some timing vendors prohibit redistribution. Those series never connect to proprietary live feeds. Final classifications are recorded only when the same facts appear in multiple independent public sources.

Policy is enforced at runtime. If a series is paused or an endpoint is disabled for that series, the API returns `403` with codes such as `series_disabled`, `endpoint_disabled_for_series`, or `live_disabled_for_series`.

## What data is available and why

Only timing and result facts are distributed: times, positions, speeds, sectors, penalties, classifications, weather. Creative media (audio, video, photos, logos, articles, graphic assets) is never redistributed, proxied, or cached.

| Class | Examples |
| --- | --- |
| Full detail | Laps, sectors, positions, pit, race control text, weather, results, standings |
| Metadata only | Driver/rider headshot URLs as source links when upstream provides them |
| Never | Logos, broadcast video/audio, onboard imagery, official app HTML/CSS assets |

## Tiers

| Tier | REST | Live stream |
| --- | --- | --- |
| anon | 1 rps / 30 per minute | none |
| free_key | 3 rps / 120 per minute | delayed (config) |
| supporter | 8 rps / 400 per minute | realtime |

Series-level free delay also applies. Exact limits live in config and can change without a deploy.

## Fair use

Non-commercial use only. Resale is forbidden. Do not rehost audio or imagery. Attribution is appreciated. Bot identity: `alchmist-motorsport-bot/1.0 (+https://f1.alchm.ist/bot)`.

## Status

See `openapi.yaml` for the living contract and `GET /v1/status` for live redistribution plus per-series status rows.
