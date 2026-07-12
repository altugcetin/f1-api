use serde_json::{json, Value};
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

const BASE: &str = "https://api.wrc.com/results-api";

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

pub async fn seasons() -> Value {
    get_cached(
        "wrc:seasons",
        Duration::from_secs(6 * 3600),
        &format!("{BASE}/rallyevent?order=DESC"),
    )
    .await
}

pub async fn events() -> Value {
    let body = seasons().await;
    let mapped: Vec<Value> = body
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(40)
        .map(|row| {
            json!({
                "series_key": "wrc",
                "event_key": row.get("rallyeventId").or_else(|| row.get("id")).cloned(),
                "name": row.get("name").or_else(|| row.get("eventName")).cloned().unwrap_or(json!("")),
                "country": row.get("country").cloned(),
                "date_start": row.get("startDate").cloned(),
                "date_end": row.get("finishDate").cloned(),
                "source_id": "wrc-results:events",
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn itinerary(event_key: &str) -> Value {
    get_cached(
        &format!("wrc:itinerary:{event_key}"),
        Duration::from_secs(120),
        &format!("{BASE}/rallyevent/{event_key}/itinerary"),
    )
    .await
}

pub async fn overall(event_key: &str) -> Value {
    get_cached(
        &format!("wrc:overall:{event_key}"),
        Duration::from_secs(60),
        &format!("{BASE}/rallyevent/{event_key}/results"),
    )
    .await
}
