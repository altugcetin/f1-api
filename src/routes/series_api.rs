use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::openf1;
use crate::policy::{self, is_live_endpoint};
use crate::providers::{f2f3, formula_e, indycar, motogp, nascar, results_facts, wrc};
use crate::routes::stub;
use crate::series::LegalTier;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LiveQuery {
    pub session_key: Option<i64>,
    pub event_key: Option<String>,
    pub limit: Option<usize>,
    pub until: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveQuery {
    pub until: Option<String>,
    #[allow(dead_code)]
    pub year: Option<i32>,
}

pub async fn list_series() -> Json<Value> {
    let rows: Vec<Value> = crate::series::active_public()
        .into_iter()
        .map(|row| row.public_view())
        .collect();
    Json(json!({ "series": rows, "count": rows.len() }))
}

async fn dispatch(
    state: &AppState,
    series_key: &str,
    endpoint: &str,
    query: LiveQuery,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let series = policy::resolve_series(series_key)?;
    policy::enforce_endpoint(series, endpoint)?;
    if is_live_endpoint(endpoint) {
        policy::enforce_live(state, series)?;
    }

    if series.series_key == "f1" {
        let value = match endpoint {
            "events" => openf1::meetings(query.limit.unwrap_or(40)).await,
            "sessions" => openf1::sessions(query.limit.unwrap_or(24)).await,
            "position" => openf1::position(query.session_key).await,
            "intervals" => openf1::intervals(query.session_key).await,
            "entries" | "competitors" => openf1::drivers(query.session_key).await,
            "weather" => openf1::weather(query.session_key, query.limit.unwrap_or(1)).await,
            "race_control" => {
                openf1::race_control(query.session_key, query.limit.unwrap_or(20)).await
            }
            "standings" => {
                return stub::standings_drivers_json()
                    .await
                    .map_err(|status| {
                        (
                            status,
                            Json(json!({
                                "error": {
                                    "code": "upstream_error",
                                    "message": "standings upstream failed"
                                }
                            })),
                        )
                    });
            }
            "results" | "laps" | "stints" | "pit" => json!([]),
            "location" | "car_data" => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": {
                            "code": "not_found",
                            "message": "no data for this resource yet"
                        }
                    })),
                ));
            }
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key.starts_with("moto") {
        let until = query
            .until
            .clone()
            .unwrap_or_else(|| "2026-07-11".to_string());
        let value = match endpoint {
            "events" => motogp::events(&series.series_key).await,
            "sessions" => motogp::sessions(&series.series_key).await,
            "results" => motogp::results(&series.series_key).await,
            "standings" => motogp::standings(&series.series_key).await,
            "archive" => motogp::archive_races(&series.series_key, &until).await,
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key == "wrc" {
        let event_key = query.event_key.as_deref().unwrap_or("");
        let value = match endpoint {
            "events" => wrc::events().await,
            "itinerary" if !event_key.is_empty() => wrc::itinerary(event_key).await,
            "overall" if !event_key.is_empty() => wrc::overall(event_key).await,
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key == "formula-e" {
        let value = match endpoint {
            "events" => formula_e::events().await,
            "standings" => formula_e::standings().await,
            "results" | "sessions" | "entries" | "competitors" => json!([]),
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key == "f2" || series.series_key == "f3" {
        let event_key = query.event_key.as_deref();
        let until = query
            .until
            .clone()
            .unwrap_or_else(|| "2026-07-11".to_string());
        let value = match endpoint {
            "events" => f2f3::events(&series.series_key).await,
            "sessions" => f2f3::sessions(&series.series_key).await,
            "standings" => f2f3::standings(&series.series_key).await,
            "results" => f2f3::results(&series.series_key, event_key).await,
            "entries" | "competitors" => f2f3::entries(&series.series_key).await,
            "archive" => f2f3::archive_races(&series.series_key, &until).await,
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key == "nascar-cup" {
        let event_key = query.event_key.as_deref();
        let until = query
            .until
            .clone()
            .unwrap_or_else(|| "2026-07-11".to_string());
        let value = match endpoint {
            "events" => nascar::events().await,
            "sessions" => nascar::sessions().await,
            "standings" => nascar::standings().await,
            "results" => nascar::results(event_key).await,
            "position" => nascar::position().await,
            "entries" | "competitors" => nascar::entries().await,
            "archive" => nascar::archive_races(&until).await,
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.series_key == "indycar" {
        let event_key = query.event_key.as_deref();
        let until = query
            .until
            .clone()
            .unwrap_or_else(|| "2026-07-11".to_string());
        let value = match endpoint {
            "events" => indycar::events().await,
            "sessions" => indycar::sessions().await,
            "standings" => indycar::standings().await,
            "results" => indycar::results(event_key).await,
            "entries" | "competitors" => indycar::entries().await,
            "archive" => indycar::archive_races(&until).await,
            _ => json!([]),
        };
        return Ok(Json(value));
    }

    if series.legal_tier == LegalTier::T3 {
        if endpoint == "results" {
            return Ok(Json(results_facts::for_series(&series.series_key)));
        }
        return Ok(Json(json!({
            "series_key": series.series_key,
            "endpoint": endpoint,
            "results": [],
            "note": "results-only series; facts come from multi-source public records",
            "source_id": format!("manual-facts:{}:{}", series.series_key, endpoint)
        })));
    }

    Ok(Json(json!([])))
}

macro_rules! series_route {
    ($name:ident, $endpoint:expr) => {
        async fn $name(
            State(state): State<AppState>,
            Path(series): Path<String>,
            Query(query): Query<LiveQuery>,
        ) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
            dispatch(&state, &series, $endpoint, query).await
        }
    };
}

series_route!(events, "events");
series_route!(sessions, "sessions");
series_route!(entries, "entries");
series_route!(competitors, "competitors");
series_route!(results, "results");
series_route!(standings, "standings");
series_route!(laps, "laps");
series_route!(stints, "stints");
series_route!(pit, "pit");
series_route!(intervals, "intervals");
series_route!(position, "position");
series_route!(race_control, "race_control");
series_route!(weather, "weather");
series_route!(location, "location");
series_route!(car_data, "car_data");
series_route!(itinerary, "itinerary");
series_route!(stages, "stages");
series_route!(stage_times, "stage_times");
series_route!(split_times, "split_times");
series_route!(overall, "overall");
series_route!(penalties, "penalties");
series_route!(retirements, "retirements");

async fn archive_races(
    State(state): State<AppState>,
    Path(series): Path<String>,
    Query(query): Query<ArchiveQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let until = query.until.unwrap_or_else(|| "2026-07-11".to_string());
    dispatch(
        &state,
        &series,
        "archive",
        LiveQuery {
            session_key: None,
            event_key: None,
            limit: None,
            until: Some(until),
        },
    )
    .await
}

async fn archive_race_detail(
    State(_state): State<AppState>,
    Path((series, event_key)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let series_row = policy::resolve_series(&series)?;
    policy::enforce_endpoint(series_row, "archive")?;
    let value = if series_row.series_key == "f1" {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "code": "not_found",
                    "message": "use /v1/archive/races/{session_key} for Formula 1"
                }
            })),
        ));
    } else if series_row.series_key == "f2" || series_row.series_key == "f3" {
        f2f3::archive_detail(&series_row.series_key, &event_key).await
    } else if series_row.series_key == "nascar-cup" {
        nascar::archive_detail(&event_key).await
    } else if series_row.series_key == "indycar" {
        indycar::archive_detail(&event_key).await
    } else if series_row.series_key.starts_with("moto") {
        motogp::archive_detail(&series_row.series_key, &event_key).await
    } else {
        json!({})
    };
    if value.as_object().map(|obj| obj.is_empty()).unwrap_or(false) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "code": "not_found",
                    "message": "archive race not found"
                }
            })),
        ));
    }
    Ok(Json(value))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/series", get(list_series))
        .route("/{series}/events", get(events))
        .route("/{series}/sessions", get(sessions))
        .route("/{series}/entries", get(entries))
        .route("/{series}/competitors", get(competitors))
        .route("/{series}/results", get(results))
        .route("/{series}/standings", get(standings))
        .route("/{series}/standings/drivers", get(standings))
        .route("/{series}/archive/races", get(archive_races))
        .route("/{series}/archive/races/{event_key}", get(archive_race_detail))
        .route("/{series}/laps", get(laps))
        .route("/{series}/stints", get(stints))
        .route("/{series}/pit", get(pit))
        .route("/{series}/intervals", get(intervals))
        .route("/{series}/position", get(position))
        .route("/{series}/race_control", get(race_control))
        .route("/{series}/weather", get(weather))
        .route("/{series}/location", get(location))
        .route("/{series}/car_data", get(car_data))
        .route("/{series}/itinerary", get(itinerary))
        .route("/{series}/stages", get(stages))
        .route("/{series}/stage_times", get(stage_times))
        .route("/{series}/split_times", get(split_times))
        .route("/{series}/overall", get(overall))
        .route("/{series}/penalties", get(penalties))
        .route("/{series}/retirements", get(retirements))
}
