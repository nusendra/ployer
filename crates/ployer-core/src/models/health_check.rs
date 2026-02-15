use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub id: String,
    pub application_id: String,
    pub path: String,
    pub interval_seconds: i32,
    pub timeout_seconds: i32,
    pub healthy_threshold: i32,
    pub unhealthy_threshold: i32,
    pub created_at: DateTime<Utc>,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            id: String::new(),
            application_id: String::new(),
            path: "/".to_string(),
            interval_seconds: 30,
            timeout_seconds: 5,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub id: String,
    pub application_id: String,
    pub container_id: String,
    pub status: HealthCheckStatus,
    pub response_time_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl HealthCheckStatus {
    pub fn as_str(&self) -> &str {
        match self {
            HealthCheckStatus::Healthy => "healthy",
            HealthCheckStatus::Unhealthy => "unhealthy",
            HealthCheckStatus::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "healthy" => HealthCheckStatus::Healthy,
            "unhealthy" => HealthCheckStatus::Unhealthy,
            _ => HealthCheckStatus::Unknown,
        }
    }
}
