use crate::state::AppState;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};

async fn not_implemented() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": {
                "code": "not_found",
                "message": "endpoint scaffolded; implementation arrives in later milestones"
            }
        })),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/meetings", get(not_implemented))
        .route("/sessions", get(not_implemented))
        .route("/drivers", get(not_implemented))
        .route("/laps", get(not_implemented))
        .route("/stints", get(not_implemented))
        .route("/pit", get(not_implemented))
        .route("/intervals", get(not_implemented))
        .route("/position", get(not_implemented))
        .route("/location", get(not_implemented))
        .route("/car_data", get(not_implemented))
        .route("/race_control", get(not_implemented))
        .route("/weather", get(not_implemented))
        .route("/team_radio", get(not_implemented))
        .route("/results", get(not_implemented))
        .route("/standings/drivers", get(not_implemented))
        .route("/standings/constructors", get(not_implemented))
        .route("/circuits/{circuit_key}/geometry", get(not_implemented))
}
