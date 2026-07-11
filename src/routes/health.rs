use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "database": state.has_database(),
        "redis": state.has_redis()
    }))
}

pub async fn metrics() -> String {
    String::new()
}
