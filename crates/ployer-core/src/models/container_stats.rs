use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub id: String,
    pub container_id: String,
    pub application_id: Option<String>,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_limit_mb: Option<f64>,
    pub network_rx_mb: Option<f64>,
    pub network_tx_mb: Option<f64>,
    pub recorded_at: DateTime<Utc>,
}
