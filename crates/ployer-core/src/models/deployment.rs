use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub application_id: String,
    pub server_id: String,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub status: DeploymentStatus,
    pub build_log: Option<String>,
    pub container_id: Option<String>,
    pub image_tag: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Queued,
    Cloning,
    Building,
    Deploying,
    Running,
    Failed,
    Cancelled,
    RolledBack,
}

impl DeploymentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            DeploymentStatus::Queued => "queued",
            DeploymentStatus::Cloning => "cloning",
            DeploymentStatus::Building => "building",
            DeploymentStatus::Deploying => "deploying",
            DeploymentStatus::Running => "running",
            DeploymentStatus::Failed => "failed",
            DeploymentStatus::Cancelled => "cancelled",
            DeploymentStatus::RolledBack => "rolled_back",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "queued" => DeploymentStatus::Queued,
            "cloning" => DeploymentStatus::Cloning,
            "building" => DeploymentStatus::Building,
            "deploying" => DeploymentStatus::Deploying,
            "running" => DeploymentStatus::Running,
            "failed" => DeploymentStatus::Failed,
            "cancelled" => DeploymentStatus::Cancelled,
            "rolled_back" => DeploymentStatus::RolledBack,
            _ => DeploymentStatus::Queued,
        }
    }
}

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
