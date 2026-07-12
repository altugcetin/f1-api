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

pub async fn events(series_key: &str) -> Value {
    let Some(category) = category_slug(series_key) else {
        return json!([]);
    };
    let seasons = get_cached(
        "motogp:seasons",
        Duration::from_secs(6 * 3600),
        &format!("{BASE}/results/seasons"),
    )
    .await;
    let year = seasons
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("year").and_then(|v| v.as_i64()))
        .unwrap_or_else(|| chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026));

    let events = get_cached(
        &format!("motogp:events:{year}"),
        Duration::from_secs(3600),
        &format!("{BASE}/results/events?seasonYear={year}"),
    )
    .await;

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
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let Some(event_id) = event_id else {
        return json!([]);
    };
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
    let seasons = get_cached(
        "motogp:seasons",
        Duration::from_secs(6 * 3600),
        &format!("{BASE}/results/seasons"),
    )
    .await;
    let year = seasons
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("year").and_then(|v| v.as_i64()))
        .unwrap_or(2026);
    let category = category_slug(series_key).unwrap_or("MotoGP");
    let body = get_cached(
        &format!("motogp:standings:{series_key}:{year}"),
        Duration::from_secs(1800),
        &format!("{BASE}/results/standings?seasonYear={year}&category={category}"),
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
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let Some(session_id) = session_id else {
        return json!([]);
    };
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
