pub mod health;
pub mod auth;
pub mod servers;
pub mod containers;
pub mod applications;
pub mod deployments;
pub mod domains;
pub mod webhooks;
pub mod monitoring;

use axum::{routing::get, Router};
use crate::app_state::SharedState;
use crate::websocket;

pub fn api_router() -> Router<SharedState> {
    Router::new()
        .nest("/health", health::router())
        .nest("/auth", auth::router())
        .nest("/servers", servers::router())
        .nest("/containers", containers::router())
        .nest("/networks", containers::networks_router())
        .nest("/volumes", containers::volumes_router())
        .nest("/applications", applications::router())
        .merge(deployments::app_deploy_router())
        .merge(domains::router())
        .merge(webhooks::router())
        .merge(monitoring::router())
        .nest("/deployments", deployments::router())
        .route("/ws", get(websocket::websocket_handler))
}
