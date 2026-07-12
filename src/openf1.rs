use chrono::{DateTime, Datelike, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use crate::response_cache;

const OPENF1_BASE: &str = "https://api.openf1.org/v1";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(crate::providers::http::BOT_UA)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn get_json(url: &str) -> Option<Value> {
    let response = client().get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<Value>().await.ok()
}

fn parse_dt(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let raw = value?.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Prefer an in-progress session; otherwise the most recently started one.
pub async fn resolve_session_key(explicit: Option<i64>) -> Option<i64> {
    if let Some(key) = explicit {
        return Some(key);
    }
    let cache_key = "openf1:active_session";
    if let Some(cached) = response_cache::get(cache_key, Duration::from_secs(20)) {
        return cached.get("session_key").and_then(|v| v.as_i64());
    }
    let now = Utc::now();
    let year = now.year();
    let mut best: Option<(DateTime<Utc>, i64, bool)> = None;
    for y in [year, year - 1] {
        let url = format!("{OPENF1_BASE}/sessions?year={y}");
        let Some(body) = get_json(&url).await else {
            continue;
        };
        let Some(rows) = body.as_array() else {
            continue;
        };
        for row in rows {
            let Some(session_key) = row.get("session_key").and_then(|v| v.as_i64()) else {
                continue;
            };
            let Some(start) = parse_dt(row.get("date_start")) else {
                continue;
            };
            let end = parse_dt(row.get("date_end"));
            let live = start <= now && end.map(|e| e >= now).unwrap_or(true);
            let replace = match best {
                None => true,
                Some((best_start, _, best_live)) => {
                    if live && !best_live {
                        true
                    } else if live == best_live {
                        start > best_start
                    } else {
                        false
                    }
                }
            };
            if replace {
                best = Some((start, session_key, live));
            }
        }
    }
    let session_key = best.map(|(_, key, _)| key)?;
    response_cache::set(
        cache_key.to_string(),
        json!({ "session_key": session_key }),
    );
    Some(session_key)
}

pub async fn sessions(limit: usize) -> Value {
    let cache_key = format!("openf1:sessions:{limit}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(30)) {
        return cached;
    }
    let now = Utc::now();
    let year = now.year();
    let mut mapped: Vec<Value> = Vec::new();
    for y in [year, year - 1] {
        let url = format!("{OPENF1_BASE}/sessions?year={y}");
        let Some(body) = get_json(&url).await else {
            continue;
        };
        let Some(rows) = body.as_array() else {
            continue;
        };
        for row in rows {
            let start = parse_dt(row.get("date_start"));
            let end = parse_dt(row.get("date_end"));
            let status = match (start, end) {
                (Some(s), Some(e)) if s <= now && e >= now => "live",
                (Some(s), _) if s > now => "upcoming",
                _ => "finished",
            };
            mapped.push(json!({
                "session_key": row.get("session_key"),
                "meeting_key": row.get("meeting_key"),
                "name": row.get("session_name").or_else(|| row.get("session_type")).unwrap_or(&json!("")),
                "type": row.get("session_type").and_then(|v| v.as_str()).unwrap_or("").to_lowercase(),
                "status": status,
                "date_start": row.get("date_start"),
                "date_end": row.get("date_end"),
                "circuit_key": row.get("circuit_key"),
                "circuit_short_name": row.get("circuit_short_name"),
                "country_name": row.get("country_name"),
                "year": row.get("year"),
            }));
        }
    }
    mapped.sort_by(|a, b| {
        let da = a.get("date_start").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date_start").and_then(|v| v.as_str()).unwrap_or("");
        db.cmp(da)
    });
    mapped.truncate(limit.max(1));
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn drivers(session_key: Option<i64>) -> Value {
    let Some(session_key) = resolve_session_key(session_key).await else {
        return json!([]);
    };
    let cache_key = format!("openf1:drivers:{session_key}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(45)) {
        return cached;
    }
    let url = format!("{OPENF1_BASE}/drivers?session_key={session_key}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };
    let mapped: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "session_key": session_key,
                "driver_number": row.get("driver_number"),
                "tla": row.get("name_acronym"),
                "full_name": row.get("full_name"),
                "first_name": row.get("first_name"),
                "last_name": row.get("last_name"),
                "team_name": row.get("team_name"),
                "team_colour": row.get("team_colour"),
                "headshot_url": row.get("headshot_url"),
                "country_code": row.get("country_code"),
            })
        })
        .collect();
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn position(session_key: Option<i64>) -> Value {
    let Some(session_key) = resolve_session_key(session_key).await else {
        return json!([]);
    };
    let cache_key = format!("openf1:position:{session_key}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(4)) {
        return cached;
    }
    let url = format!("{OPENF1_BASE}/position?session_key={session_key}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };

    let mut latest: HashMap<i64, &Value> = HashMap::new();
    for row in rows {
        let Some(driver_number) = row.get("driver_number").and_then(|v| v.as_i64()) else {
            continue;
        };
        let date = row.get("date").and_then(|v| v.as_str()).unwrap_or("");
        match latest.get(&driver_number) {
            Some(existing) => {
                let existing_date = existing.get("date").and_then(|v| v.as_str()).unwrap_or("");
                if date > existing_date {
                    latest.insert(driver_number, row);
                }
            }
            None => {
                latest.insert(driver_number, row);
            }
        }
    }

    let intervals_url = format!("{OPENF1_BASE}/intervals?session_key={session_key}");
    let interval_body = get_json(&intervals_url).await;
    let mut gap_by_driver: HashMap<i64, String> = HashMap::new();
    if let Some(Value::Array(interval_rows)) = interval_body.as_ref() {
        let mut latest_interval: HashMap<i64, &Value> = HashMap::new();
        for row in interval_rows {
            let Some(driver_number) = row.get("driver_number").and_then(|v| v.as_i64()) else {
                continue;
            };
            let date = row.get("date").and_then(|v| v.as_str()).unwrap_or("");
            match latest_interval.get(&driver_number) {
                Some(existing) => {
                    let existing_date =
                        existing.get("date").and_then(|v| v.as_str()).unwrap_or("");
                    if date > existing_date {
                        latest_interval.insert(driver_number, row);
                    }
                }
                None => {
                    latest_interval.insert(driver_number, row);
                }
            }
        }
        for (driver_number, row) in latest_interval {
            let gap = row
                .get("gap_to_leader")
                .cloned()
                .or_else(|| row.get("interval").cloned());
            if let Some(gap) = gap {
                let text = match gap {
                    Value::Number(n) => format!("+{:.3}", n.as_f64().unwrap_or(0.0)),
                    Value::String(s) => s,
                    _ => String::new(),
                };
                if !text.is_empty() {
                    gap_by_driver.insert(driver_number, text);
                }
            }
        }
    }

    let mut mapped: Vec<Value> = latest
        .into_iter()
        .map(|(driver_number, row)| {
            let gap = gap_by_driver
                .get(&driver_number)
                .cloned()
                .unwrap_or_else(|| {
                    if row.get("position").and_then(|v| v.as_i64()) == Some(1) {
                        "LEADER".into()
                    } else {
                        "-".into()
                    }
                });
            json!({
                "session_key": session_key,
                "driver_number": driver_number,
                "position": row.get("position"),
                "date": row.get("date"),
                "gap_to_leader": gap,
                "last_lap": Value::Null,
            })
        })
        .collect();
    mapped.sort_by_key(|row| {
        row.get("position")
            .and_then(|v| v.as_i64())
            .unwrap_or(99)
    });
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn weather(session_key: Option<i64>, limit: usize) -> Value {
    let Some(session_key) = resolve_session_key(session_key).await else {
        return json!([]);
    };
    let cache_key = format!("openf1:weather:{session_key}:{limit}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(15)) {
        return cached;
    }
    let url = format!("{OPENF1_BASE}/weather?session_key={session_key}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };
    let mut mapped: Vec<Value> = rows
        .iter()
        .rev()
        .take(limit.max(1))
        .map(|row| {
            json!({
                "session_key": session_key,
                "date": row.get("date"),
                "air_temp": row.get("air_temperature"),
                "track_temp": row.get("track_temperature"),
                "humidity": row.get("humidity"),
                "pressure": row.get("pressure"),
                "rainfall": row.get("rainfall"),
                "wind_speed": row.get("wind_speed"),
                "wind_direction": row.get("wind_direction"),
            })
        })
        .collect();
    mapped.reverse();
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn race_control(session_key: Option<i64>, limit: usize) -> Value {
    let Some(session_key) = resolve_session_key(session_key).await else {
        return json!([]);
    };
    let cache_key = format!("openf1:rc:{session_key}:{limit}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(8)) {
        return cached;
    }
    let url = format!("{OPENF1_BASE}/race_control?session_key={session_key}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };
    let mapped: Vec<Value> = rows
        .iter()
        .rev()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, row)| {
            json!({
                "id": format!(
                    "{}-{index}",
                    row.get("date").and_then(|v| v.as_str()).unwrap_or("")
                ),
                "session_key": session_key,
                "date": row.get("date"),
                "category": row.get("category"),
                "flag": row.get("flag"),
                "scope": row.get("scope"),
                "message": row.get("message"),
                "driver_number": row.get("driver_number"),
                "lap_number": row.get("lap_number"),
            })
        })
        .collect();
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn intervals(session_key: Option<i64>) -> Value {
    let Some(session_key) = resolve_session_key(session_key).await else {
        return json!([]);
    };
    let cache_key = format!("openf1:intervals:{session_key}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(4)) {
        return cached;
    }
    let url = format!("{OPENF1_BASE}/intervals?session_key={session_key}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };
    let mut latest: HashMap<i64, &Value> = HashMap::new();
    for row in rows {
        let Some(driver_number) = row.get("driver_number").and_then(|v| v.as_i64()) else {
            continue;
        };
        let date = row.get("date").and_then(|v| v.as_str()).unwrap_or("");
        match latest.get(&driver_number) {
            Some(existing) => {
                let existing_date = existing.get("date").and_then(|v| v.as_str()).unwrap_or("");
                if date > existing_date {
                    latest.insert(driver_number, row);
                }
            }
            None => {
                latest.insert(driver_number, row);
            }
        }
    }
    let mapped: Vec<Value> = latest
        .into_iter()
        .map(|(driver_number, row)| {
            json!({
                "session_key": session_key,
                "driver_number": driver_number,
                "gap_to_leader": row.get("gap_to_leader"),
                "interval": row.get("interval"),
                "date": row.get("date"),
            })
        })
        .collect();
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}

pub async fn meetings(limit: usize) -> Value {
    let cache_key = format!("openf1:meetings:{limit}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(60)) {
        return cached;
    }
    let year = Utc::now().year();
    let url = format!("{OPENF1_BASE}/meetings?year={year}");
    let Some(body) = get_json(&url).await else {
        return json!([]);
    };
    let Some(rows) = body.as_array() else {
        return json!([]);
    };
    let mapped: Vec<Value> = rows
        .iter()
        .rev()
        .take(limit.max(1))
        .map(|row| {
            json!({
                "meeting_key": row.get("meeting_key"),
                "name": row.get("meeting_name"),
                "circuit_key": row.get("circuit_key"),
                "circuit_short_name": row.get("circuit_short_name"),
                "country_name": row.get("country_name"),
                "date_start": row.get("date_start"),
                "year": row.get("year"),
            })
        })
        .collect();
    let payload = Value::Array(mapped);
    response_cache::set(cache_key, payload.clone());
    payload
}
