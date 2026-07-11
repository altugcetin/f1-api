use crate::state::AppState;
use api_types::StatusResponse;
use axum::extract::State;
use axum::Json;

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(state.status_response())
}
