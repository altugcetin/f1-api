use serde_json::{json, Value};
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

const BASE: &str = "https://api.motogp.pulselive.com/motogp/v1";

fn category_slug(series_key: &str) -> Option<&'static str> {
    match series_key {
        "motogp" => Some("MotoGP"),
        "moto2" => Some("Moto2"),
        "moto3" => Some("Moto3"),
        "motoe" => Some("MotoE"),
        _ => None,
    }
}

async fn get_cached(cache_key: &str, ttl: Duration, url: &str) -> Value {
    if let Some(cached) = response_cache::get(cache_key, ttl) {
        return cached;
    }
    let Ok(response) = client().get(url).send().await else {
        return json!([]);
    };
    if !response.status().is_success() {
        return json!([]);
    }
    let Ok(body) = response.json::<Value>().await else {
        return json!([]);
    };
    response_cache::set(cache_key.to_string(), body.clone());
    body
}

async fn seasons() -> Value {
    get_cached(
        "motogp:seasons",
        Duration::from_secs(6 * 3600),
        &format!("{BASE}/results/seasons"),
    )
    .await
}

fn season_uuid_for_year(seasons: &Value, year: i64) -> Option<String> {
    seasons.as_array()?.iter().find_map(|row| {
        if row.get("year").and_then(|v| v.as_i64()) == Some(year) {
            row.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    })
}

async fn events_for_year(year: i64) -> Value {
    let seasons = seasons().await;
    let Some(uuid) = season_uuid_for_year(&seasons, year) else {
        return json!([]);
    };
    get_cached(
        &format!("motogp:events:{year}:{uuid}"),
        Duration::from_secs(3600),
        &format!("{BASE}/results/events?seasonUuid={uuid}"),
    )
    .await
}

pub async fn events(series_key: &str) -> Value {
    let Some(category) = category_slug(series_key) else {
        return json!([]);
    };
    let seasons = seasons().await;
    let year = seasons
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("year").and_then(|v| v.as_i64()))
        .unwrap_or(2026);
    let events = events_for_year(year).await;
    let mapped: Vec<Value> = events
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": series_key,
                "event_key": row.get("id").cloned().unwrap_or(json!(null)),
                "name": row.get("name").or_else(|| row.get("short_name")).cloned().unwrap_or(json!("")),
                "country": row.get("country").cloned().unwrap_or(json!(null)),
                "date_start": row.get("date_start").or_else(|| row.get("start_date")).cloned(),
                "date_end": row.get("date_end").or_else(|| row.get("end_date")).cloned(),
                "category": category,
                "source_id": format!("pulselive:events:{year}"),
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn sessions(series_key: &str) -> Value {
    let events = events(series_key).await;
    let Some(event) = events.as_array().and_then(|rows| rows.first()) else {
        return json!([]);
    };
    let event_id = event
        .get("event_key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if event_id.is_empty() {
        return json!([]);
    }
    let category = category_slug(series_key).unwrap_or("MotoGP");
    let body = get_cached(
        &format!("motogp:sessions:{series_key}:{event_id}"),
        Duration::from_secs(600),
        &format!("{BASE}/results/sessions?event={event_id}&category={category}"),
    )
    .await;
    let mapped: Vec<Value> = body
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": series_key,
                "session_key": row.get("id").cloned().unwrap_or(json!(null)),
                "event_key": event_id,
                "name": row.get("name").or_else(|| row.get("type")).cloned().unwrap_or(json!("")),
                "type": row.get("type").cloned().unwrap_or(json!("")),
                "date_start": row.get("date_start").or_else(|| row.get("date")).cloned(),
                "status": row.get("status").cloned().unwrap_or(json!("")),
                "source_id": format!("pulselive:sessions:{event_id}"),
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn standings(series_key: &str) -> Value {
    let seasons = seasons().await;
    let year = seasons
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("year").and_then(|v| v.as_i64()))
        .unwrap_or(2026);
    let category = category_slug(series_key).unwrap_or("MotoGP");
    let Some(uuid) = season_uuid_for_year(&seasons, year) else {
        return json!({
            "series_key": series_key,
            "season": year,
            "standings": [],
            "source_id": format!("pulselive:standings:{category}:{year}")
        });
    };
    let body = get_cached(
        &format!("motogp:standings:{series_key}:{year}"),
        Duration::from_secs(1800),
        &format!("{BASE}/results/standings?seasonUuid={uuid}&category={category}"),
    )
    .await;
    json!({
        "series_key": series_key,
        "season": year,
        "standings": body,
        "source_id": format!("pulselive:standings:{category}:{year}")
    })
}

pub async fn results(series_key: &str) -> Value {
    let sessions = sessions(series_key).await;
    let Some(session) = sessions.as_array().and_then(|rows| rows.last()) else {
        return json!([]);
    };
    let session_id = session
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if session_id.is_empty() {
        return json!([]);
    }
    let body = get_cached(
        &format!("motogp:class:{session_id}"),
        Duration::from_secs(300),
        &format!("{BASE}/results/session/{session_id}/classification"),
    )
    .await;
    json!({
        "series_key": series_key,
        "session_key": session_id,
        "classification": body,
        "source_id": format!("pulselive:classification:{session_id}")
    })
}

pub async fn archive_races(series_key: &str, until: &str) -> Value {
    let Some(category) = category_slug(series_key) else {
        return json!([]);
    };
    let years = crate::providers::archive_common::archive_years(until);
    let mut out = Vec::new();
    for year in years {
        let events = events_for_year(year as i64).await;
        for (idx, row) in events
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            if row.get("test").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            let date = crate::providers::archive_common::date_only(
                row.get("date_end")
                    .or_else(|| row.get("date_start"))
                    .and_then(|v| v.as_str()),
            );
            if !crate::providers::archive_common::finished(&date, until) {
                continue;
            }
            let event_id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if event_id.is_empty() {
                continue;
            }
            let name = row
                .get("name")
                .or_else(|| row.get("short_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Round");
            let country = row
                .pointer("/country/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let circuit = row
                .pointer("/circuit/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            out.push(crate::providers::archive_common::archive_race(
                series_key,
                &format!("{series_key}-{event_id}"),
                year,
                (idx + 1) as i32,
                name,
                &date,
                circuit,
                "",
                country,
                json!([{
                    "session_key": format!("{series_key}-{event_id}"),
                    "type": category,
                    "name": "Race",
                    "date": date,
                    "time": Value::Null,
                    "has_replay": false
                }]),
            ));
        }
    }
    out.sort_by(|a, b| {
        let da = a.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date").and_then(|v| v.as_str()).unwrap_or("");
        db.cmp(da)
    });
    Value::Array(out)
}

pub async fn archive_detail(series_key: &str, event_key: &str) -> Value {
    let event_id = event_key
        .trim_start_matches(&format!("{series_key}-"))
        .to_string();
    let races = archive_races(series_key, "2026-12-31").await;
    let race = races
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row.get("session_key").and_then(|v| v.as_str()) == Some(event_key))
        .cloned()
        .unwrap_or_else(|| {
            crate::providers::archive_common::archive_race(
                series_key,
                event_key,
                0,
                0,
                "Round",
                "",
                "",
                "",
                "",
                json!([]),
            )
        });
    let category = category_slug(series_key).unwrap_or("MotoGP");
    let sessions = get_cached(
        &format!("motogp:sessions:{series_key}:{event_id}"),
        Duration::from_secs(3600),
        &format!("{BASE}/results/sessions?event={event_id}&category={category}"),
    )
    .await;
    let session_id = sessions
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find_map(|row| {
            let name = row
                .get("name")
                .or_else(|| row.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            if name.contains("race") {
                row.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            sessions.as_array().and_then(|rows| {
                rows.last()
                    .and_then(|row| row.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            })
        });
    let mut results = Vec::new();
    if let Some(session_id) = session_id {
        let body = get_cached(
            &format!("motogp:class:{session_id}"),
            Duration::from_secs(3600),
            &format!("{BASE}/results/session/{session_id}/classification"),
        )
        .await;
        let rows = body
            .as_array()
            .cloned()
            .or_else(|| body.get("classification").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();
        results = rows
            .into_iter()
            .enumerate()
            .map(|(idx, row)| {
                let rider = row.get("rider").cloned().unwrap_or_else(|| row.clone());
                let full = rider
                    .get("full_name")
                    .or_else(|| rider.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let (given, family) = if let Some((g, f)) = full.split_once(' ') {
                    (g.to_string(), f.to_string())
                } else {
                    (full.clone(), String::new())
                };
                json!({
                    "position": row.get("position").cloned().unwrap_or(json!(idx + 1)),
                    "position_text": row.get("position").map(|v| v.to_string()).unwrap_or_else(|| (idx + 1).to_string()),
                    "points": row.get("points").cloned(),
                    "grid": Value::Null,
                    "laps": row.get("laps").cloned(),
                    "status": row.get("status").cloned().unwrap_or(json!("Finished")),
                    "time": row.get("time").cloned(),
                    "driver_id": rider.get("id").map(|v| json!(v.to_string())).unwrap_or(json!(format!("rider-{idx}"))),
                    "tla": family.chars().take(3).collect::<String>().to_uppercase(),
                    "driver_number": rider.get("number").cloned(),
                    "given_name": given,
                    "family_name": family,
                    "constructor_id": row.get("team").and_then(|v| v.as_str()).map(|s| s.to_lowercase().replace(' ', "-")).unwrap_or_default(),
                    "constructor_name": row.get("team").cloned().unwrap_or(json!("")),
                    "fastest_lap": Value::Null,
                    "fastest_lap_number": Value::Null,
                })
            })
            .collect();
    }
    crate::providers::archive_common::archive_detail(
        race,
        "Race",
        results,
        &format!("pulselive:archive:{event_key}"),
    )
}
