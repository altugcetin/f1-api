use crate::state::AppState;
use api_types::StatusResponse;
use axum::extract::State;
use axum::Json;

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut response = state.status_response();
    if let Some(session_key) = crate::openf1::resolve_session_key(None).await {
        response.active_session_key = Some(session_key);
        if response.feed_latency_ms.is_none() {
            response.feed_latency_ms = Some(0);
        }
    }
    Json(response)
}
