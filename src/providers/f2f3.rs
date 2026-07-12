use serde_json::{json, Value};
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

fn host(series_key: &str) -> Option<&'static str> {
    match series_key {
        "f2" => Some("https://www.fiaformula2.com"),
        "f3" => Some("https://www.fiaformula3.com"),
        _ => None,
    }
}

async fn get_html(cache_key: &str, ttl: Duration, url: &str) -> Option<String> {
    if let Some(cached) = response_cache::get(cache_key, ttl) {
        return cached.as_str().map(|s| s.to_string());
    }
    let Ok(response) = client().get(url).send().await else {
        return None;
    };
    if !response.status().is_success() {
        return None;
    }
    let Ok(body) = response.text().await else {
        return None;
    };
    response_cache::set(cache_key.to_string(), json!(body));
    Some(body)
}

fn extract_next_data(html: &str) -> Option<Value> {
    let marker = r#"<script id="__NEXT_DATA__" type="application/json">"#;
    let start = html.find(marker)? + marker.len();
    let end = html[start..].find("</script>")? + start;
    serde_json::from_str(&html[start..end]).ok()
}

fn page_data(next: &Value) -> Option<&Value> {
    next.pointer("/props/pageProps/pageData")
}

pub async fn events(series_key: &str) -> Value {
    let Some(base) = host(series_key) else {
        return json!([]);
    };
    let html = get_html(
        &format!("{series_key}:calendar:html"),
        Duration::from_secs(3600),
        &format!("{base}/Calendar"),
    )
    .await;
    let Some(html) = html else {
        return json!([]);
    };
    let Some(next) = extract_next_data(&html) else {
        return json!([]);
    };
    let Some(page) = page_data(&next) else {
        return json!([]);
    };
    let mapped: Vec<Value> = page
        .get("Races")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": series_key,
                "event_key": row.get("RaceId").map(|v| json!(v.to_string())),
                "name": format!(
                    "{} {}",
                    row.get("CircuitShortName").and_then(|v| v.as_str()).unwrap_or("Round"),
                    row.get("CountryName").and_then(|v| v.as_str()).unwrap_or("")
                ).trim().to_string(),
                "circuit_name": row.get("CircuitName").cloned().unwrap_or(json!("")),
                "country": row.get("CountryName").or_else(|| row.get("CountryCode")).cloned(),
                "locality": row.get("CircuitShortName").cloned(),
                "date_start": row.get("RaceStartDate").cloned(),
                "date_end": row.get("RaceEndDate").cloned(),
                "round": row.get("RoundNumber").cloned(),
                "sessions": row.get("Sessions").cloned().unwrap_or(json!([])),
                "source_id": format!("{series_key}:calendar"),
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn sessions(series_key: &str) -> Value {
    let events = events(series_key).await;
    let mut out = Vec::new();
    for event in events.as_array().cloned().unwrap_or_default() {
        let event_key = event.get("event_key").cloned().unwrap_or(json!(null));
        for session in event
            .get("sessions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            out.push(json!({
                "series_key": series_key,
                "session_key": session.get("SessionId").cloned(),
                "event_key": event_key.clone(),
                "name": session.get("SessionName").cloned().unwrap_or(json!("")),
                "type": session.get("SessionCode").cloned(),
                "date_start": session.get("SessionStartTime").cloned(),
                "date_end": session.get("SessionEndTime").cloned(),
                "source_id": format!("{series_key}:sessions"),
            }));
        }
    }
    Value::Array(out)
}

pub async fn standings(series_key: &str) -> Value {
    let Some(base) = host(series_key) else {
        return json!({});
    };
    let html = get_html(
        &format!("{series_key}:standings:html"),
        Duration::from_secs(1800),
        &format!("{base}/Standings/Driver"),
    )
    .await;
    let Some(html) = html else {
        return json!({ "series_key": series_key, "standings": [], "source_id": format!("{series_key}:standings") });
    };
    let Some(next) = extract_next_data(&html) else {
        return json!({ "series_key": series_key, "standings": [], "source_id": format!("{series_key}:standings") });
    };
    let page = page_data(&next).cloned().unwrap_or(json!({}));
    let rows: Vec<Value> = page
        .get("Standings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            let full = row
                .get("FullName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (given, family) = if let Some((g, f)) = full.split_once(' ') {
                (g.to_string(), f.to_string())
            } else {
                (full.clone(), String::new())
            };
            json!({
                "position": row.get("Position").cloned(),
                "points": row.get("TotalPoints").cloned(),
                "wins": Value::Null,
                "driver_id": row.get("DriverID").map(|v| json!(v.to_string())),
                "tla": row.get("TLA").cloned().unwrap_or(json!("")),
                "given_name": given,
                "family_name": family,
                "constructor_id": row.get("TeamName").and_then(|v| v.as_str()).map(|s| {
                    s.to_lowercase().replace(' ', "-")
                }).unwrap_or_default(),
                "constructor_name": row.get("TeamName").cloned().unwrap_or(json!("")),
                "car_number": row.get("CarNumber").cloned(),
            })
        })
        .collect();
    json!({
        "series_key": series_key,
        "season": page.get("Season").cloned(),
        "standings": rows,
        "source_id": format!("{series_key}:standings")
    })
}

pub async fn results(series_key: &str, event_key: Option<&str>) -> Value {
    let Some(base) = host(series_key) else {
        return json!([]);
    };
    let race_id = match event_key {
        Some(key) => key.to_string(),
        None => {
            let events = events(series_key).await;
            let Some(last) = events.as_array().and_then(|rows| {
                rows.iter()
                    .rev()
                    .find(|row| {
                        row.get("sessions")
                            .and_then(|v| v.as_array())
                            .map(|sessions| {
                                sessions.iter().any(|s| {
                                    s.get("SessionResultsAvailable")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false)
                                        || s.get("WinnerId").is_some()
                                })
                            })
                            .unwrap_or(false)
                    })
                    .or_else(|| rows.last())
            }) else {
                return json!([]);
            };
            last.get("event_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        }
    };
    if race_id.is_empty() {
        return json!([]);
    }
    let html = get_html(
        &format!("{series_key}:results:{race_id}"),
        Duration::from_secs(900),
        &format!("{base}/Results?raceid={race_id}"),
    )
    .await;
    let Some(html) = html else {
        return json!([]);
    };
    let Some(next) = extract_next_data(&html) else {
        return json!([]);
    };
    let page = page_data(&next).cloned().unwrap_or(json!({}));
    let sessions: Vec<Value> = page
        .get("SessionResults")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|session| {
            let classification: Vec<Value> = session
                .get("Results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    json!({
                        "position": row.get("FinishPosition").cloned(),
                        "position_text": row.get("DisplayFinishPosition").cloned(),
                        "driver_id": row.get("DriverId").map(|v| json!(v.to_string())),
                        "tla": row.get("TLA").cloned(),
                        "given_name": row.get("DriverForename").cloned(),
                        "family_name": row.get("DriverSurname").cloned(),
                        "driver_number": row.get("CarNumber").cloned(),
                        "constructor_name": row.get("TeamName").cloned(),
                        "constructor_id": row.get("TeamName").and_then(|v| v.as_str()).map(|s| {
                            s.to_lowercase().replace(' ', "-")
                        }),
                        "laps": row.get("LapsCompleted").cloned(),
                        "time": row.get("TimeOrFinishReason").cloned(),
                        "status": row.get("ResultStatus").cloned().unwrap_or(json!("Finished")),
                        "points": Value::Null,
                        "grid": Value::Null,
                        "fastest_lap": row.get("Best").cloned(),
                    })
                })
                .collect();
            json!({
                "session_key": session.get("SessionId").cloned(),
                "name": session.get("SessionName").cloned(),
                "type": session.get("SessionType").or_else(|| session.get("SessionShortName")).cloned(),
                "classification": classification,
            })
        })
        .collect();
    json!({
        "series_key": series_key,
        "event_key": race_id,
        "name": format!(
            "{} {}",
            page.get("CircuitInformation")
                .and_then(|c| c.get("CircuitShortName"))
                .and_then(|v| v.as_str())
                .or_else(|| page.get("CountryName").and_then(|v| v.as_str()))
                .unwrap_or("Round"),
            page.get("CountryName").and_then(|v| v.as_str()).unwrap_or("")
        ).trim(),
        "circuit_name": page
            .pointer("/CircuitInformation/CircuitName")
            .cloned()
            .unwrap_or(json!("")),
        "country": page.get("CountryName").cloned(),
        "date_start": page.get("RaceStartDate").cloned(),
        "date_end": page.get("RaceEndDate").cloned(),
        "round": page.get("RoundNumber").cloned(),
        "sessions": sessions,
        "source_id": format!("{series_key}:results:{race_id}")
    })
}

pub async fn entries(series_key: &str) -> Value {
    let standings = standings(series_key).await;
    let rows: Vec<Value> = standings
        .get("standings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "driver_number": row.get("car_number").cloned(),
                "full_name": format!(
                    "{} {}",
                    row.get("given_name").and_then(|v| v.as_str()).unwrap_or(""),
                    row.get("family_name").and_then(|v| v.as_str()).unwrap_or("")
                ).trim(),
                "name_acronym": row.get("tla").cloned(),
                "team_name": row.get("constructor_name").cloned(),
                "broadcast_name": row.get("family_name").cloned(),
            })
        })
        .collect();
    Value::Array(rows)
}
