pub mod user;
pub mod server;
pub mod application;
pub mod deployment;
pub mod domain;

pub use user::*;
pub use server::*;
pub use application::*;
pub use deployment::*;
pub use domain::*;

use serde::{Deserialize, Serialize};

/// WebSocket event broadcast to connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    DeploymentStatus {
        deployment_id: String,
        app_id: String,
        status: DeploymentStatus,
    },
    DeploymentLog {
        deployment_id: String,
        line: String,
    },
    ContainerStats {
        container_id: String,
        cpu_percent: f64,
        memory_mb: f64,
    },
    ServerHealth {
        server_id: String,
        status: ServerStatus,
    },
}
