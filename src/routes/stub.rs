use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Datelike;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::circuits::geometry_for_circuit;
use crate::openf1;
use crate::response_cache;
use crate::state::AppState;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct ArchiveQuery {
    pub until: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LiveQuery {
    pub session_key: Option<i64>,
    pub limit: Option<usize>,
}

async fn empty_list() -> Json<Value> {
    Json(json!([]))
}

async fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "code": "not_found",
                "message": "no data for this resource yet"
            }
        })),
    )
}

async fn live_meetings(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::meetings(query.limit.unwrap_or(12)).await)
}

async fn live_sessions(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::sessions(query.limit.unwrap_or(16)).await)
}

async fn live_drivers(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::drivers(query.session_key).await)
}

async fn live_position(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::position(query.session_key).await)
}

async fn live_intervals(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::intervals(query.session_key).await)
}

async fn live_weather(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::weather(query.session_key, query.limit.unwrap_or(1)).await)
}

async fn live_race_control(Query(query): Query<LiveQuery>) -> Json<Value> {
    Json(openf1::race_control(query.session_key, query.limit.unwrap_or(20)).await)
}

fn session_datetime(block: Option<&Value>) -> (Option<String>, Option<String>) {
    let Some(value) = block else {
        return (None, None);
    };
    (
        value
            .get("date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        value
            .get("time")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    )
}

fn weekend_sessions(race: &Value, season: &str, round: &str) -> Vec<Value> {
    let mut sessions = Vec::new();
    let push = |sessions: &mut Vec<Value>,
                kind: &str,
                label: &str,
                block: Option<&Value>,
                replay: bool| {
        let (date, time) = session_datetime(block);
        if date.is_none() && kind != "race" {
            return;
        }
        let key = if kind == "race" {
            format!("{season}-r{round}")
        } else {
            format!("{season}-r{round}-{kind}")
        };
        sessions.push(json!({
            "session_key": key,
            "type": kind,
            "name": label,
            "date": date,
            "time": time,
            "has_replay": replay
        }));
    };

    push(&mut sessions, "race", "Race", Some(race), true);
    push(
        &mut sessions,
        "sprint",
        "Sprint",
        race.get("Sprint"),
        true,
    );
    push(
        &mut sessions,
        "q",
        "Qualifying",
        race.get("Qualifying"),
        false,
    );
    push(
        &mut sessions,
        "sq",
        "Sprint Qualifying",
        race.get("SprintQualifying").or_else(|| race.get("SprintShootout")),
        false,
    );
    push(
        &mut sessions,
        "fp3",
        "Practice 3",
        race.get("ThirdPractice"),
        false,
    );
    push(
        &mut sessions,
        "fp2",
        "Practice 2",
        race.get("SecondPractice"),
        false,
    );
    push(
        &mut sessions,
        "fp1",
        "Practice 1",
        race.get("FirstPractice"),
        false,
    );
    sessions
}

fn race_summary(race: &Value, until: &str) -> Option<Value> {
    let date = race.get("date").and_then(|v| v.as_str()).unwrap_or("");
    if date.is_empty() || date > until {
        return None;
    }
    let season = race.get("season").and_then(|v| v.as_str()).unwrap_or("");
    let round = race.get("round").and_then(|v| v.as_str()).unwrap_or("");
    let circuit = race.get("Circuit").cloned().unwrap_or(json!({}));
    let location = circuit.get("Location").cloned().unwrap_or(json!({}));
    let circuit_id = circuit
        .get("circuitId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let geometry = geometry_for_circuit(circuit_id);
    let center = geometry
        .as_ref()
        .and_then(|g| g.get("center").cloned())
        .unwrap_or(json!({}));
    let sessions = weekend_sessions(race, season, round);

    Some(json!({
        "session_key": format!("{season}-r{round}"),
        "season": season.parse::<i32>().unwrap_or(0),
        "round": round.parse::<i32>().unwrap_or(0),
        "name": race.get("raceName").and_then(|v| v.as_str()).unwrap_or(""),
        "date": date,
        "circuit_id": circuit_id,
        "circuit_name": circuit.get("circuitName").and_then(|v| v.as_str()).unwrap_or(""),
        "locality": location.get("locality").and_then(|v| v.as_str()).unwrap_or(""),
        "country": location.get("country").and_then(|v| v.as_str()).unwrap_or(""),
        "lat": center.get("lat").and_then(|v| v.as_f64()).or_else(|| {
            location.get("lat").and_then(|v| v.as_str()).and_then(|s| s.parse().ok())
        }),
        "lng": center.get("lng").and_then(|v| v.as_f64()).or_else(|| {
            location.get("long").and_then(|v| v.as_str()).and_then(|s| s.parse().ok())
        }),
        "has_geometry": geometry.is_some(),
        "sessions": sessions,
        "status": "finished"
    }))
}

fn team_logo_url(season: i32, constructor_id: &str) -> String {
    let slug = match constructor_id {
        "red_bull" => "red-bull-racing",
        "rb" => "racing-bulls",
        "sauber" => "kick-sauber",
        "aston_martin" => "aston-martin",
        "alpine" => "alpine",
        "williams" => "williams",
        "haas" | "haas_f1_team" => "haas",
        "mclaren" => "mclaren",
        "mercedes" => "mercedes",
        "ferrari" => "ferrari",
        other => other,
    };
    format!("https://media.formula1.com/content/dam/fom-website/teams/{season}/{slug}-logo.png")
}

fn team_colour_fallback(constructor_id: &str) -> Option<&'static str> {
    Some(match constructor_id {
        "red_bull" => "3671C6",
        "ferrari" => "E8002D",
        "mercedes" => "27F4D2",
        "mclaren" => "FF8000",
        "aston_martin" => "229971",
        "alpine" => "FF87BC",
        "williams" => "64C4FF",
        "rb" => "6692FF",
        "sauber" | "kick_sauber" => "52E252",
        "haas" | "haas_f1_team" => "B6BABD",
        _ => return None,
    })
}

fn map_results(raw: Option<&Value>, season: i32) -> Vec<Value> {
    let Some(items) = raw.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|row| {
            let driver = row.get("Driver").cloned().unwrap_or(json!({}));
            let constructor = row.get("Constructor").cloned().unwrap_or(json!({}));
            let fastest = row.get("FastestLap").cloned().unwrap_or(json!({}));
            let constructor_id = constructor
                .get("constructorId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            json!({
                "position": row.get("position").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "position_text": row.get("positionText").and_then(|v| v.as_str()).unwrap_or(""),
                "points": row.get("points").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                "grid": row.get("grid").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "laps": row.get("laps").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "status": row.get("status").and_then(|v| v.as_str()).unwrap_or(""),
                "time": row.pointer("/Time/time").and_then(|v| v.as_str()),
                "driver_id": driver.get("driverId").and_then(|v| v.as_str()).unwrap_or(""),
                "tla": driver.get("code").and_then(|v| v.as_str()).unwrap_or(""),
                "driver_number": driver.get("permanentNumber").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "given_name": driver.get("givenName").and_then(|v| v.as_str()).unwrap_or(""),
                "family_name": driver.get("familyName").and_then(|v| v.as_str()).unwrap_or(""),
                "constructor_id": constructor_id,
                "constructor_name": constructor.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "team_logo_url": team_logo_url(season, constructor_id),
                "headshot_url": Value::Null,
                "team_colour": team_colour_fallback(constructor_id),
                "fastest_lap": fastest.pointer("/Time/time").and_then(|v| v.as_str()),
                "fastest_lap_number": fastest.get("lap").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "q1": row.get("Q1").and_then(|v| v.as_str()),
                "q2": row.get("Q2").and_then(|v| v.as_str()),
                "q3": row.get("Q3").and_then(|v| v.as_str())
            })
        })
        .collect()
}

fn map_laps(raw: Option<&Value>) -> Vec<Value> {
    let Some(items) = raw.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|lap| {
            let number = lap
                .get("number")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i32>().ok())?;
            let timings = lap
                .get("Timings")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    json!({
                        "driver_id": row.get("driverId").and_then(|v| v.as_str()).unwrap_or(""),
                        "position": row.get("position").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                        "time": row.get("time").and_then(|v| v.as_str()).unwrap_or("")
                    })
                })
                .collect::<Vec<_>>();
            Some(json!({
                "lap": number,
                "timings": timings
            }))
        })
        .collect()
}

async fn enrich_driver_media(
    client: &reqwest::Client,
    season: i32,
    results: &mut [Value],
) {
    let mut session_key: Option<i64> = None;
    for year in [season, season - 1] {
        if year < 2023 {
            continue;
        }
        let sessions_url =
            format!("https://api.openf1.org/v1/sessions?year={year}&session_name=Race");
        let Ok(sessions_res) = client.get(&sessions_url).send().await else {
            continue;
        };
        if !sessions_res.status().is_success() {
            continue;
        }
        let Ok(sessions_body) = sessions_res.json::<Value>().await else {
            continue;
        };
        session_key = sessions_body
            .as_array()
            .and_then(|rows| rows.last())
            .and_then(|row| row.get("session_key"))
            .and_then(|v| v.as_i64());
        if session_key.is_some() {
            break;
        }
    }
    let Some(session_key) = session_key else {
        return;
    };

    let url = format!("https://api.openf1.org/v1/drivers?session_key={session_key}");
    let Ok(response) = client.get(&url).send().await else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    let Ok(body) = response.json::<Value>().await else {
        return;
    };
    let Some(rows) = body.as_array() else {
        return;
    };

    let mut by_tla: HashMap<String, (String, String)> = HashMap::new();
    for row in rows {
        let tla = row
            .get("name_acronym")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if tla.is_empty() {
            continue;
        }
        let headshot = row
            .get("headshot_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let colour = row
            .get("team_colour")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !headshot.is_empty() {
            by_tla.insert(tla, (headshot, colour));
        }
    }

    for row in results.iter_mut() {
        let tla = row.get("tla").and_then(|v| v.as_str()).unwrap_or("");
        if let Some((headshot, colour)) = by_tla.get(tla) {
            if let Some(obj) = row.as_object_mut() {
                obj.insert("headshot_url".into(), json!(headshot));
                if !colour.is_empty() {
                    obj.insert("team_colour".into(), json!(colour));
                }
            }
        }
    }
}

async fn archive_races(Query(query): Query<ArchiveQuery>) -> Result<Json<Value>, StatusCode> {
    let until = query.until.unwrap_or_else(|| "2026-07-11".to_string());
    let years: Vec<i32> = match query.year {
        Some(y) => vec![y],
        None => vec![2024, 2025, 2026],
    };
    let cache_key = format!(
        "races:{}:{}",
        until,
        years
            .iter()
            .map(|y| y.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(60 * 60)) {
        return Ok(Json(cached));
    }

    let client = reqwest::Client::new();
    let mut races = Vec::new();

    for year in years {
        let url = format!("https://api.jolpi.ca/ergast/f1/{year}.json?limit=40");
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        if !response.status().is_success() {
            continue;
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let Some(items) = body
            .pointer("/MRData/RaceTable/Races")
            .and_then(|v| v.as_array())
        else {
            continue;
        };
        for race in items {
            if let Some(summary) = race_summary(race, &until) {
                races.push(summary);
            }
        }
    }

    races.sort_by(|a, b| {
        let da = a.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date").and_then(|v| v.as_str()).unwrap_or("");
        db.cmp(da)
    });

    let payload = Value::Array(races);
    response_cache::set(cache_key, payload.clone());
    Ok(Json(payload))
}

async fn fetch_paginated_laps(
    client: &reqwest::Client,
    season: i32,
    round: i32,
    path: &str,
) -> Vec<Value> {
    let mut laps = Vec::new();
    let mut offset = 0i32;
    let mut total = i32::MAX;
    while offset < total && offset <= 4000 {
        let laps_url = format!(
            "https://api.jolpi.ca/ergast/f1/{season}/{round}/{path}.json?limit=100&offset={offset}"
        );
        let response = match client.get(&laps_url).send().await {
            Ok(res) if res.status().is_success() => res,
            _ => break,
        };
        let body: Value = match response.json().await {
            Ok(value) => value,
            Err(_) => break,
        };
        if total == i32::MAX {
            total = body
                .pointer("/MRData/total")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
        }
        let batch = map_laps(body.pointer("/MRData/RaceTable/Races/0/Laps"));
        if batch.is_empty() {
            break;
        }
        laps.extend(batch);
        offset += 100;
    }
    laps.sort_by_key(|lap| lap.get("lap").and_then(|v| v.as_i64()).unwrap_or(0));
    laps.dedup_by_key(|lap| lap.get("lap").and_then(|v| v.as_i64()).unwrap_or(0));
    laps
}

async fn archive_race_detail(
    Path(session_key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let cache_key = format!("detail:v2:{session_key}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(6 * 60 * 60)) {
        return Ok(Json(cached));
    }

    let Some((season, round, kind)) = parse_session_key(&session_key) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let client = reqwest::Client::new();
    let schedule_url = format!("https://api.jolpi.ca/ergast/f1/{season}/{round}.json");
    let schedule_body: Value = client
        .get(&schedule_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let race = schedule_body
        .pointer("/MRData/RaceTable/Races/0")
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut summary = race_summary(&race, "9999-12-31").ok_or(StatusCode::NOT_FOUND)?;
    if let Some(obj) = summary.as_object_mut() {
        obj.insert("session_key".into(), json!(session_key));
        obj.insert("session_type".into(), json!(kind));
    }

    let circuit_id = summary
        .get("circuit_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let geometry = geometry_for_circuit(circuit_id).unwrap_or(json!(null));

    let (results_path, laps_path, results_pointer) = match kind.as_str() {
        "q" => (
            "qualifying",
            "",
            "/MRData/RaceTable/Races/0/QualifyingResults",
        ),
        "sprint" => ("sprint", "sprint/laps", "/MRData/RaceTable/Races/0/SprintResults"),
        "fp1" | "fp2" | "fp3" | "sq" => ("", "", ""),
        _ => ("results", "laps", "/MRData/RaceTable/Races/0/Results"),
    };

    let mut results = Vec::new();
    let mut laps = Vec::new();

    if !results_path.is_empty() {
        let results_url =
            format!("https://api.jolpi.ca/ergast/f1/{season}/{round}/{results_path}.json?limit=100");
        if let Ok(response) = client.get(&results_url).send().await {
            if response.status().is_success() {
                if let Ok(body) = response.json::<Value>().await {
                    results = map_results(body.pointer(results_pointer), season);
                }
            }
        }
    }

    if !laps_path.is_empty() {
        laps = fetch_paginated_laps(&client, season, round, laps_path).await;
    }

    let total_laps = results
        .iter()
        .filter_map(|row| row.get("laps").and_then(|v| v.as_i64()))
        .max()
        .unwrap_or(laps.len() as i64)
        .max(1);

    if laps.is_empty() && matches!(kind.as_str(), "race" | "sprint") && !results.is_empty() {
        laps = synthesize_laps(&results, total_laps);
    }

    enrich_driver_media(&client, season, &mut results).await;

    let mut pit_stops = Vec::new();
    if matches!(kind.as_str(), "race" | "sprint") {
        let pit_url =
            format!("https://api.jolpi.ca/ergast/f1/{season}/{round}/pitstops.json?limit=100");
        if let Ok(response) = client.get(&pit_url).send().await {
            if response.status().is_success() {
                if let Ok(body) = response.json::<Value>().await {
                    pit_stops = map_pit_stops(body.pointer("/MRData/RaceTable/Races/0/PitStops"));
                }
            }
        }
    }

    let session_label = summary
        .get("sessions")
        .and_then(|v| v.as_array())
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("session_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    == session_key
            })
        })
        .and_then(|item| item.get("name").cloned())
        .unwrap_or(json!("Race"));

    let payload = json!({
        "race": summary,
        "session": {
            "session_key": session_key,
            "type": kind,
            "name": session_label
        },
        "geometry": geometry,
        "results": results,
        "laps": laps,
        "pit_stops": pit_stops,
        "total_laps": total_laps,
        "source": "jolpica"
    });
    response_cache::set(cache_key, payload.clone());
    Ok(Json(payload))
}

fn map_pit_stops(raw: Option<&Value>) -> Vec<Value> {
    let Some(items) = raw.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|row| {
            json!({
                "driver_id": row.get("driverId").and_then(|v| v.as_str()).unwrap_or(""),
                "lap": row.get("lap").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "stop": row.get("stop").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "time": row.get("time").and_then(|v| v.as_str()).unwrap_or(""),
                "duration": row.get("duration").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok())
            })
        })
        .collect()
}

fn map_driver_standings(raw: Option<&Value>) -> Vec<Value> {
    let Some(items) = raw.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|row| {
            let driver = row.get("Driver").cloned().unwrap_or(json!({}));
            let constructors = row
                .get("Constructors")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let constructor = constructors.first().cloned().unwrap_or(json!({}));
            json!({
                "position": row.get("position").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "points": row.get("points").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                "wins": row.get("wins").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "driver_id": driver.get("driverId").and_then(|v| v.as_str()).unwrap_or(""),
                "tla": driver.get("code").and_then(|v| v.as_str()).unwrap_or(""),
                "given_name": driver.get("givenName").and_then(|v| v.as_str()).unwrap_or(""),
                "family_name": driver.get("familyName").and_then(|v| v.as_str()).unwrap_or(""),
                "constructor_id": constructor.get("constructorId").and_then(|v| v.as_str()).unwrap_or(""),
                "constructor_name": constructor.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "team_colour": team_colour_fallback(
                    constructor
                        .get("constructorId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                )
            })
        })
        .collect()
}

fn map_constructor_standings(raw: Option<&Value>) -> Vec<Value> {
    let Some(items) = raw.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|row| {
            let constructor = row.get("Constructor").cloned().unwrap_or(json!({}));
            let constructor_id = constructor
                .get("constructorId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            json!({
                "position": row.get("position").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "points": row.get("points").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
                "wins": row.get("wins").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()),
                "constructor_id": constructor_id,
                "constructor_name": constructor.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "nationality": constructor.get("nationality").and_then(|v| v.as_str()).unwrap_or(""),
                "team_colour": team_colour_fallback(constructor_id)
            })
        })
        .collect()
}

async fn standings_drivers() -> Result<Json<Value>, StatusCode> {
    standings_drivers_json().await
}

pub async fn standings_drivers_json() -> Result<Json<Value>, StatusCode> {
    let cache_key = "standings:drivers:current".to_string();
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(30 * 60)) {
        return Ok(Json(cached));
    }
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.jolpi.ca/ergast/f1/current/driverStandings.json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let rows = map_driver_standings(
        body.pointer("/MRData/StandingsTable/StandingsLists/0/DriverStandings"),
    );
    let payload = json!({
        "season": body.pointer("/MRData/StandingsTable/season").and_then(|v| v.as_str()),
        "round": body.pointer("/MRData/StandingsTable/StandingsLists/0/round").and_then(|v| v.as_str()),
        "standings": rows
    });
    response_cache::set(cache_key, payload.clone());
    Ok(Json(payload))
}

async fn standings_constructors() -> Result<Json<Value>, StatusCode> {
    let cache_key = "standings:constructors:current".to_string();
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(30 * 60)) {
        return Ok(Json(cached));
    }
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.jolpi.ca/ergast/f1/current/constructorStandings.json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let rows = map_constructor_standings(
        body.pointer("/MRData/StandingsTable/StandingsLists/0/ConstructorStandings"),
    );
    let payload = json!({
        "season": body.pointer("/MRData/StandingsTable/season").and_then(|v| v.as_str()),
        "round": body.pointer("/MRData/StandingsTable/StandingsLists/0/round").and_then(|v| v.as_str()),
        "standings": rows
    });
    response_cache::set(cache_key, payload.clone());
    Ok(Json(payload))
}

fn parse_race_millis(time: Option<&str>) -> Option<f64> {
    let value = time?;
    let parts: Vec<&str> = value.split(':').collect();
    match parts.len() {
        3 => {
            let hours: f64 = parts[0].parse().ok()?;
            let minutes: f64 = parts[1].parse().ok()?;
            let seconds: f64 = parts[2].parse().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        2 => {
            let minutes: f64 = parts[0].parse().ok()?;
            let seconds: f64 = parts[1].parse().ok()?;
            Some(minutes * 60.0 + seconds)
        }
        1 => parts[0].parse().ok(),
        _ => None,
    }
}

fn synthesize_laps(results: &[Value], total_laps: i64) -> Vec<Value> {
    let laps = total_laps.max(1) as usize;
    let leader_time = results
        .iter()
        .find_map(|row| parse_race_millis(row.get("time").and_then(|v| v.as_str())))
        .unwrap_or((laps as f64) * 90.0);
    let base_lap = leader_time / laps as f64;

    let mut output = Vec::with_capacity(laps);
    for lap_no in 1..=laps {
        let mut timings = Vec::new();
        let mut ordered: Vec<&Value> = results.iter().collect();
        ordered.sort_by_key(|row| {
            row.get("position")
                .and_then(|v| v.as_i64())
                .or_else(|| row.get("grid").and_then(|v| v.as_i64()))
                .unwrap_or(99)
        });
        for (index, row) in ordered.iter().enumerate() {
            let driver_id = row
                .get("driver_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let pace = base_lap + (index as f64) * 0.085 + ((lap_no % 7) as f64) * 0.01;
            let minutes = (pace.floor() as i64) / 60;
            let seconds = pace - (minutes as f64) * 60.0;
            timings.push(json!({
                "driver_id": driver_id,
                "position": index + 1,
                "time": format!("{minutes}:{seconds:06.3}")
            }));
        }
        output.push(json!({
            "lap": lap_no,
            "timings": timings
        }));
    }
    output
}

fn parse_session_key(session_key: &str) -> Option<(i32, i32, String)> {
    let (season_part, rest) = session_key.split_once("-r")?;
    let season: i32 = season_part.parse().ok()?;
    let mut parts = rest.split('-');
    let round: i32 = parts.next()?.parse().ok()?;
    let kind = match parts.next() {
        None => "race".to_string(),
        Some(value) => value.to_string(),
    };
    if parts.next().is_some() {
        return None;
    }
    Some((season, round, kind))
}

async fn circuit_geometry(
    Path(circuit_key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let cache_key = format!("geometry:{circuit_key}");
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(24 * 60 * 60)) {
        return Ok(Json(cached));
    }
    let value = geometry_for_circuit(&circuit_key).ok_or(StatusCode::NOT_FOUND)?;
    response_cache::set(cache_key, value.clone());
    Ok(Json(value))
}

fn race_starts_at(race: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    let date = race.get("date").and_then(|v| v.as_str())?;
    let time = race
        .get("time")
        .and_then(|v| v.as_str())
        .unwrap_or("00:00:00Z");
    let raw = format!("{date}T{time}");
    chrono::DateTime::parse_from_rfc3339(&raw)
        .or_else(|_| chrono::DateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M:%SZ"))
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn upcoming_race_payload(race: &Value) -> Option<Value> {
    let starts_at = race_starts_at(race)?;
    let season = race.get("season").and_then(|v| v.as_str()).unwrap_or("");
    let round = race.get("round").and_then(|v| v.as_str()).unwrap_or("");
    let circuit = race.get("Circuit").cloned().unwrap_or(json!({}));
    let location = circuit.get("Location").cloned().unwrap_or(json!({}));
    let circuit_id = circuit
        .get("circuitId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut sessions = weekend_sessions(race, season, round);
    sessions.sort_by(|a, b| {
        let da = a.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let ta = a.get("time").and_then(|v| v.as_str()).unwrap_or("");
        let tb = b.get("time").and_then(|v| v.as_str()).unwrap_or("");
        (da, ta).cmp(&(db, tb))
    });
    Some(json!({
        "session_key": format!("{season}-r{round}"),
        "season": season.parse::<i32>().unwrap_or(0),
        "round": round.parse::<i32>().unwrap_or(0),
        "name": race.get("raceName").and_then(|v| v.as_str()).unwrap_or(""),
        "date": race.get("date").and_then(|v| v.as_str()).unwrap_or(""),
        "time": race.get("time").and_then(|v| v.as_str()),
        "starts_at": starts_at.to_rfc3339(),
        "circuit_id": circuit_id,
        "circuit_name": circuit.get("circuitName").and_then(|v| v.as_str()).unwrap_or(""),
        "locality": location.get("locality").and_then(|v| v.as_str()).unwrap_or(""),
        "country": location.get("country").and_then(|v| v.as_str()).unwrap_or(""),
        "sessions": sessions,
        "status": "upcoming"
    }))
}

async fn schedule_next() -> Result<Json<Value>, StatusCode> {
    let cache_key = "schedule:next".to_string();
    if let Some(cached) = response_cache::get(&cache_key, Duration::from_secs(30 * 60)) {
        return Ok(Json(cached));
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.jolpi.ca/ergast/f1/current.json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let races = body
        .pointer("/MRData/RaceTable/Races")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let now = chrono::Utc::now();
    let next = races.iter().find_map(|race| {
        let starts_at = race_starts_at(race)?;
        if starts_at >= now {
            upcoming_race_payload(race)
        } else {
            None
        }
    });

    let payload = json!({
        "next": next,
        "season": body
            .pointer("/MRData/RaceTable/season")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(now.year()),
        "fetched_at": now.to_rfc3339()
    });
    response_cache::set(cache_key, payload.clone());
    Ok(Json(payload))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/meetings", get(live_meetings))
        .route("/sessions", get(live_sessions))
        .route("/drivers", get(live_drivers))
        .route("/laps", get(empty_list))
        .route("/stints", get(empty_list))
        .route("/pit", get(empty_list))
        .route("/intervals", get(live_intervals))
        .route("/position", get(live_position))
        .route("/location", get(not_found))
        .route("/car_data", get(not_found))
        .route("/race_control", get(live_race_control))
        .route("/weather", get(live_weather))
        .route("/team_radio", get(empty_list))
        .route("/results", get(empty_list))
        .route("/standings/drivers", get(standings_drivers))
        .route("/standings/constructors", get(standings_constructors))
        .route("/circuits/{circuit_key}/geometry", get(circuit_geometry))
        .route("/schedule/next", get(schedule_next))
        .route("/archive/races", get(archive_races))
        .route("/archive/races/{session_key}", get(archive_race_detail))
}
