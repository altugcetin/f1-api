use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::series::{self, SeriesRecord, SeriesStatus};
use crate::state::AppState;

pub type PolicyResult = Result<(), (StatusCode, Json<Value>)>;

fn deny(code: &str, message: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        })),
    )
}

pub fn resolve_series(series_key: &str) -> Result<&'static SeriesRecord, (StatusCode, Json<Value>)> {
    match series::get(series_key) {
        Some(row) => Ok(row),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "code": "series_not_found",
                    "message": format!("unknown series '{series_key}'")
                }
            })),
        )),
    }
}

pub fn enforce_series(series: &SeriesRecord) -> PolicyResult {
    match series.status {
        SeriesStatus::Active => Ok(()),
        SeriesStatus::Paused => Err(deny(
            "series_disabled",
            &format!("series '{}' is paused", series.series_key),
        )),
        SeriesStatus::Excluded => Err(deny(
            "series_disabled",
            &format!("series '{}' is excluded", series.series_key),
        )),
    }
}

pub fn enforce_endpoint(series: &SeriesRecord, endpoint: &str) -> PolicyResult {
    enforce_series(series)?;
    if series
        .enabled_endpoints
        .iter()
        .any(|item| item == endpoint)
    {
        Ok(())
    } else {
        Err(deny(
            "endpoint_disabled_for_series",
            &format!(
                "endpoint '{endpoint}' is not enabled for series '{}'",
                series.series_key
            ),
        ))
    }
}

pub fn enforce_live(state: &AppState, series: &SeriesRecord) -> PolicyResult {
    enforce_series(series)?;
    if !state.live_redistribution_enabled {
        return Err(deny(
            "live_disabled",
            "global live redistribution is disabled",
        ));
    }
    if !series.live_enabled {
        return Err(deny(
            "live_disabled_for_series",
            &format!("live data is disabled for series '{}'", series.series_key),
        ));
    }
    Ok(())
}

pub const LIVE_ENDPOINTS: &[&str] = &[
    "position",
    "intervals",
    "laps",
    "stints",
    "pit",
    "race_control",
    "weather",
    "location",
    "car_data",
];

pub fn is_live_endpoint(endpoint: &str) -> bool {
    LIVE_ENDPOINTS.contains(&endpoint)
}
