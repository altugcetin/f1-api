use serde_json::{json, Value};
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

const BASE: &str = "https://api.formula-e.pulselive.com/formulae/v1";

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

pub async fn events() -> Value {
    let body = get_cached(
        "fe:events",
        Duration::from_secs(3600),
        &format!("{BASE}/results/racingrounds"),
    )
    .await;
    let mapped: Vec<Value> = body
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": "formula-e",
                "event_key": row.get("id").cloned(),
                "name": row.get("name").cloned().unwrap_or(json!("")),
                "date_start": row.get("date").or_else(|| row.get("startDate")).cloned(),
                "source_id": "pulselive-fe:events",
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn standings() -> Value {
    get_cached(
        "fe:standings",
        Duration::from_secs(1800),
        &format!("{BASE}/results/standings"),
    )
    .await
}
