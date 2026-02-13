pub mod health;
pub mod auth;

use axum::Router;
use crate::app_state::SharedState;

pub fn api_router() -> Router<SharedState> {
    Router::new()
        .nest("/health", health::router())
        .nest("/auth", auth::router())
}
