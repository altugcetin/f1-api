use crate::state::AppState;
use api_types::StatusResponse;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn status(State(state): State<AppState>) -> Json<Value> {
    let mut response = state.status_response();
    if let Some(session_key) = crate::openf1::resolve_session_key(None).await {
        response.active_session_key = Some(session_key);
        if response.feed_latency_ms.is_none() {
            response.feed_latency_ms = Some(0);
        }
    }

    let series: Vec<Value> = crate::series::active_public()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": row.series_key,
                "status": row.status,
                "coverage": row.coverage(),
                "live_enabled": row.live_enabled,
                "legal_tier": row.legal_tier,
            })
        })
        .collect();

    Json(json!({
        "active_session_key": response.active_session_key,
        "feed_latency_ms": response.feed_latency_ms,
        "live_redistribution_enabled": response.live_redistribution_enabled,
        "api_version": response.api_version,
        "series": series,
    }))
}

#[allow(dead_code)]
pub async fn status_typed(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(state.status_response())
}
