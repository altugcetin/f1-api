mod health;
mod series_api;
mod status;
mod stub;

pub use health::{health, metrics};
pub use status::status;

use crate::state::AppState;
use axum::Router;

pub fn v1_router() -> Router<AppState> {
    Router::new()
        .merge(series_api::router())
        .merge(stub::router())
}
