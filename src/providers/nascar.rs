use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

const SERIES_ID: i64 = 1;
const CACHE_PREFIX: &str = "nascar-cup";

fn year() -> i32 {
    chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026)
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

fn race_name(row: &Value) -> String {
    row.get("race_name")
        .and_then(|v| v.as_str())
        .or_else(|| row.get("event_name").and_then(|v| v.as_str()))
        .unwrap_or("NASCAR event")
        .to_string()
}

pub async fn events() -> Value {
    let y = year();
    let body = get_cached(
        &format!("{CACHE_PREFIX}:schedule:{y}"),
        Duration::from_secs(3600),
        &format!("https://cf.nascar.com/cacher/{y}/{SERIES_ID}/schedule-feed.json"),
    )
    .await;

    let mut by_race: BTreeMap<i64, Value> = BTreeMap::new();
    for row in body.as_array().cloned().unwrap_or_default() {
        let race_id = row
            .get("race_id")
            .and_then(|v| v.as_i64())
            .unwrap_or_default();
        if race_id == 0 {
            continue;
        }
        let run_type = row.get("run_type").and_then(|v| v.as_i64()).unwrap_or(0);
        let start = row
            .get("start_time_utc")
            .or_else(|| row.get("start_time"))
            .cloned();
        let end = row
            .get("end_time_utc")
            .or_else(|| row.get("end_time"))
            .cloned();
        let entry = by_race.entry(race_id).or_insert_with(|| {
            json!({
                "series_key": "nascar-cup",
                "event_key": race_id.to_string(),
                "name": race_name(&row),
                "circuit_name": row.get("track_name").cloned().unwrap_or(json!("")),
                "country": "USA",
                "date_start": start.clone(),
                "date_end": end.clone(),
                "round": 0,
                "source_id": format!("nascar:schedule:{y}"),
            })
        });
        if run_type == 3 || entry.get("name").and_then(|v| v.as_str()) == Some("NASCAR event") {
            entry["name"] = json!(race_name(&row));
        }
        if let Some(s) = start {
            let current = entry.get("date_start").and_then(|v| v.as_str()).unwrap_or("");
            if current.is_empty()
                || s.as_str()
                    .map(|candidate| candidate < current)
                    .unwrap_or(false)
            {
                entry["date_start"] = s;
            }
        }
        if let Some(e) = end {
            let current = entry.get("date_end").and_then(|v| v.as_str()).unwrap_or("");
            if current.is_empty()
                || e.as_str()
                    .map(|candidate| candidate > current)
                    .unwrap_or(false)
            {
                entry["date_end"] = e;
            }
        }
        if entry.get("circuit_name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            if let Some(track) = row.get("track_name") {
                entry["circuit_name"] = track.clone();
            }
        }
    }

    let mut mapped: Vec<Value> = by_race.into_values().collect();
    mapped.sort_by(|a, b| {
        let da = a.get("date_start").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date_start").and_then(|v| v.as_str()).unwrap_or("");
        da.cmp(db)
    });
    for (idx, row) in mapped.iter_mut().enumerate() {
        row["round"] = json!(idx + 1);
    }
    Value::Array(mapped)
}

pub async fn sessions() -> Value {
    let y = year();
    let body = get_cached(
        &format!("{CACHE_PREFIX}:schedule:{y}"),
        Duration::from_secs(3600),
        &format!("https://cf.nascar.com/cacher/{y}/{SERIES_ID}/schedule-feed.json"),
    )
    .await;
    let mapped: Vec<Value> = body
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": "nascar-cup",
                "session_key": row.get("race_id").cloned(),
                "event_key": row.get("race_id").map(|v| json!(v.to_string())),
                "name": row.get("event_name").or_else(|| row.get("race_name")).cloned().unwrap_or(json!("")),
                "type": row.get("run_type").cloned(),
                "date_start": row.get("start_time_utc").or_else(|| row.get("start_time")).cloned(),
                "track_name": row.get("track_name").cloned(),
                "source_id": format!("nascar:sessions:{y}"),
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn standings() -> Value {
    let y = year();
    let body = get_cached(
        &format!("{CACHE_PREFIX}:points:{y}"),
        Duration::from_secs(1800),
        &format!("https://cf.nascar.com/cacher/{y}/{SERIES_ID}/points-feed.json"),
    )
    .await;
    let mut rows: Vec<Value> = body
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let first = row
                .get("driver_first_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let last = row
                .get("driver_last_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let full = row
                .get("driver_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (given, family) = if !first.is_empty() || !last.is_empty() {
                (first.to_string(), last.to_string())
            } else if let Some((g, f)) = full.split_once(' ') {
                (g.to_string(), f.to_string())
            } else {
                (full.clone(), String::new())
            };
            let car = row
                .get("car_no")
                .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
                .unwrap_or_default();
            json!({
                "position": row.get("position").cloned(),
                "points": row.get("points").cloned(),
                "wins": row.get("wins").cloned(),
                "driver_id": row.get("driver_id").map(|v| json!(v.to_string())).unwrap_or(json!(car.clone())),
                "tla": family.chars().take(3).collect::<String>().to_uppercase(),
                "given_name": given,
                "family_name": family,
                "constructor_id": row.get("manufacturer").cloned().unwrap_or(json!("")),
                "constructor_name": row.get("manufacturer").cloned().unwrap_or(json!("")),
                "car_number": car,
            })
        })
        .collect();
    rows.sort_by_key(|row| row.get("position").and_then(|v| v.as_i64()).unwrap_or(999));
    json!({
        "series_key": "nascar-cup",
        "season": y,
        "standings": rows,
        "source_id": format!("nascar:points:{y}")
    })
}

pub async fn results(event_key: Option<&str>) -> Value {
    let events = events().await;
    let event = events
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| {
            event_key
                .map(|key| row.get("event_key").and_then(|v| v.as_str()) == Some(key))
                .unwrap_or(false)
        })
        .or_else(|| events.as_array().and_then(|rows| rows.last()))
        .cloned();
    let Some(event) = event else {
        return json!([]);
    };
    json!({
        "series_key": "nascar-cup",
        "event_key": event.get("event_key").cloned(),
        "name": event.get("name").cloned(),
        "classification": [],
        "note": "Historical race classification is not in the public schedule feed; live classification is on /position while a race is underway.",
        "source_id": "nascar:results"
    })
}

pub async fn position() -> Value {
    let body = get_cached(
        &format!("{CACHE_PREFIX}:live"),
        Duration::from_secs(3),
        "https://cf.nascar.com/live/feeds/live-feed.json",
    )
    .await;
    let series_id = body.get("series_id").and_then(|v| v.as_i64()).unwrap_or(0);
    if series_id != 0 && series_id != SERIES_ID {
        return json!([]);
    }
    let mut rows: Vec<Value> = body
        .get("vehicles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|veh| {
            let driver = veh.get("driver").cloned().unwrap_or(json!({}));
            json!({
                "position": veh.get("running_position").cloned(),
                "driver_number": veh.get("vehicle_number").cloned(),
                "name_acronym": driver.get("last_name").and_then(|v| v.as_str()).map(|s| {
                    s.chars().take(3).collect::<String>().to_uppercase()
                }),
                "full_name": driver.get("full_name").cloned(),
                "team_name": veh.get("vehicle_manufacturer").cloned(),
                "gap_to_leader": veh.get("delta").cloned(),
                "laps": veh.get("laps_completed").cloned(),
                "last_lap_time": veh.get("last_lap_time").cloned(),
                "status": veh.get("status").cloned(),
            })
        })
        .collect();
    rows.sort_by_key(|row| row.get("position").and_then(|v| v.as_i64()).unwrap_or(999));
    Value::Array(rows)
}

pub async fn entries() -> Value {
    let body = get_cached(
        &format!("{CACHE_PREFIX}:live"),
        Duration::from_secs(3),
        "https://cf.nascar.com/live/feeds/live-feed.json",
    )
    .await;
    let rows: Vec<Value> = body
        .get("vehicles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|veh| {
            let driver = veh.get("driver").cloned().unwrap_or(json!({}));
            json!({
                "driver_number": veh.get("vehicle_number").cloned(),
                "full_name": driver.get("full_name").cloned(),
                "name_acronym": driver.get("last_name").and_then(|v| v.as_str()).map(|s| {
                    s.chars().take(3).collect::<String>().to_uppercase()
                }),
                "team_name": veh.get("vehicle_manufacturer").cloned(),
                "broadcast_name": driver.get("full_name").cloned(),
            })
        })
        .collect();
    Value::Array(rows)
}
