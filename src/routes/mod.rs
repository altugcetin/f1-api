mod health;
mod status;
mod stub;

pub use health::{health, metrics};
pub use status::status;

use crate::state::AppState;
use axum::Router;

pub fn v1_router() -> Router<AppState> {
    Router::new().merge(stub::router())
}
