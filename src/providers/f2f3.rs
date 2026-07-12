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

fn season_list(next: &Value) -> Vec<Value> {
    next.pointer("/props/pageProps/seasonData")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub async fn archive_races(series_key: &str, until: &str) -> Value {
    let Some(base) = host(series_key) else {
        return json!([]);
    };
    let html = get_html(
        &format!("{series_key}:standings:html"),
        Duration::from_secs(1800),
        &format!("{base}/Standings/Driver"),
    )
    .await;
    let Some(html) = html else {
        return json!([]);
    };
    let Some(next) = extract_next_data(&html) else {
        return json!([]);
    };
    let years = crate::providers::archive_common::archive_years(until);
    let mut out = Vec::new();
    for season in season_list(&next) {
        if season
            .get("SeasonTypeCode")
            .and_then(|v| v.as_str())
            .unwrap_or("MAIN")
            != "MAIN"
        {
            continue;
        }
        let season_id = season.get("SeasonId").and_then(|v| v.as_i64());
        let Some(season_id) = season_id else {
            continue;
        };
        let season_name = season
            .get("SeasonName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let year = season_name
            .split_whitespace()
            .rev()
            .find_map(|part| part.parse::<i32>().ok())
            .or_else(|| {
                season
                    .get("SeasonStartDate")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.get(0..4)?.parse().ok())
            })
            .unwrap_or(0);
        if !years.contains(&year) {
            continue;
        }
        let season_html = get_html(
            &format!("{series_key}:standings:{season_id}"),
            Duration::from_secs(6 * 3600),
            &format!("{base}/Standings/Driver?seasonId={season_id}"),
        )
        .await;
        let Some(season_html) = season_html else {
            continue;
        };
        let Some(season_next) = extract_next_data(&season_html) else {
            continue;
        };
        let page = page_data(&season_next).cloned().unwrap_or(json!({}));
        let races = page
            .get("SeasonRaces")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for (idx, row) in races.into_iter().enumerate() {
            let date = crate::providers::archive_common::date_only(
                row.get("RaceEndDate")
                    .or_else(|| row.get("RaceStartDate"))
                    .and_then(|v| v.as_str()),
            );
            if !crate::providers::archive_common::finished(&date, until) {
                continue;
            }
            let race_id = row
                .get("RaceId")
                .map(|v| v.to_string())
                .unwrap_or_default();
            if race_id.is_empty() {
                continue;
            }
            let round = row
                .get("RoundNumber")
                .and_then(|v| v.as_i64())
                .unwrap_or((idx + 1) as i64) as i32;
            let short = row
                .get("CircuitShortName")
                .and_then(|v| v.as_str())
                .unwrap_or("Round");
            let sessions = Value::Array(
                row.get("Sessions")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|session| {
                        json!({
                            "session_key": session.get("SessionId").map(|v| v.to_string()),
                            "type": session.get("SessionShortName").cloned().unwrap_or(json!("")),
                            "name": session.get("SessionName").cloned().unwrap_or(json!("")),
                            "date": date,
                            "time": Value::Null,
                            "has_replay": false
                        })
                    })
                    .collect(),
            );
            out.push(crate::providers::archive_common::archive_race(
                series_key,
                &race_id,
                year,
                round,
                &format!("{short} {year}"),
                &date,
                short,
                short,
                "",
                sessions,
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
    let payload = results(series_key, Some(event_key)).await;
    if payload.as_array().is_some() {
        return json!({});
    }
    let sessions = payload
        .get("sessions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let preferred = sessions
        .iter()
        .rev()
        .find(|session| {
            let has = session
                .get("classification")
                .and_then(|v| v.as_array())
                .map(|rows| !rows.is_empty())
                .unwrap_or(false);
            let name = format!(
                "{} {}",
                session.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                session.get("type").and_then(|v| v.as_str()).unwrap_or("")
            )
            .to_lowercase();
            has && (name.contains("feature") || name.contains("race"))
        })
        .or_else(|| {
            sessions.iter().rev().find(|session| {
                session
                    .get("classification")
                    .and_then(|v| v.as_array())
                    .map(|rows| !rows.is_empty())
                    .unwrap_or(false)
            })
        });
    let classification = preferred
        .and_then(|session| session.get("classification").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let date = crate::providers::archive_common::date_only(
        payload
            .get("date_end")
            .or_else(|| payload.get("date_start"))
            .and_then(|v| v.as_str()),
    );
    let year = date
        .get(0..4)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let round = payload
        .get("round")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Round");
    let race = crate::providers::archive_common::archive_race(
        series_key,
        event_key,
        year,
        round,
        name,
        &date,
        payload
            .get("circuit_name")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "",
        payload.get("country").and_then(|v| v.as_str()).unwrap_or(""),
        json!(sessions
            .iter()
            .map(|session| {
                json!({
                    "session_key": session.get("session_key").map(|v| v.to_string()),
                    "type": session.get("type").cloned().unwrap_or(json!("")),
                    "name": session.get("name").cloned().unwrap_or(json!("")),
                    "date": date,
                    "time": Value::Null,
                    "has_replay": false
                })
            })
            .collect::<Vec<_>>()),
    );
    crate::providers::archive_common::archive_detail(
        race,
        preferred
            .and_then(|session| session.get("name").and_then(|v| v.as_str()))
            .unwrap_or("Race"),
        classification,
        &format!("{series_key}:archive:{event_key}"),
    )
}
