use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub git_url: Option<String>,
    pub git_branch: String,
    pub build_strategy: BuildStrategy,
    pub dockerfile_path: Option<String>,
    pub port: Option<u16>,
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<i64>,
    pub status: AppStatus,
    pub auto_deploy: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuildStrategy {
    Dockerfile,
    Nixpacks,
    DockerCompose,
}

impl Default for BuildStrategy {
    fn default() -> Self {
        BuildStrategy::Dockerfile
    }
}

impl BuildStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            BuildStrategy::Dockerfile => "dockerfile",
            BuildStrategy::Nixpacks => "nixpacks",
            BuildStrategy::DockerCompose => "docker_compose",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dockerfile" => BuildStrategy::Dockerfile,
            "nixpacks" => BuildStrategy::Nixpacks,
            "docker_compose" => BuildStrategy::DockerCompose,
            _ => BuildStrategy::Dockerfile,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Idle,
    Building,
    Running,
    Stopped,
    Failed,
}

impl AppStatus {
    pub fn as_str(&self) -> &str {
        match self {
            AppStatus::Idle => "idle",
            AppStatus::Building => "building",
            AppStatus::Running => "running",
            AppStatus::Stopped => "stopped",
            AppStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "idle" => AppStatus::Idle,
            "building" => AppStatus::Building,
            "running" => AppStatus::Running,
            "stopped" => AppStatus::Stopped,
            "failed" => AppStatus::Failed,
            _ => AppStatus::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_strategy_round_trips() {
        for (variant, s) in [
            (BuildStrategy::Dockerfile, "dockerfile"),
            (BuildStrategy::Nixpacks, "nixpacks"),
            (BuildStrategy::DockerCompose, "docker_compose"),
        ] {
            assert_eq!(variant.as_str(), s);
            assert_eq!(BuildStrategy::from_str(s), variant);
        }
    }

    #[test]
    fn build_strategy_unknown_defaults_to_dockerfile() {
        assert_eq!(BuildStrategy::from_str("unknown"), BuildStrategy::Dockerfile);
    }

    #[test]
    fn app_status_round_trips() {
        for (variant, s) in [
            (AppStatus::Idle, "idle"),
            (AppStatus::Building, "building"),
            (AppStatus::Running, "running"),
            (AppStatus::Stopped, "stopped"),
            (AppStatus::Failed, "failed"),
        ] {
            assert_eq!(variant.as_str(), s);
            assert_eq!(AppStatus::from_str(s), variant);
        }
    }

    #[test]
    fn app_status_unknown_defaults_to_idle() {
        assert_eq!(AppStatus::from_str("mystery"), AppStatus::Idle);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub id: String,
    pub application_id: String,
    pub key: String,
    pub value_encrypted: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployKey {
    pub id: String,
    pub application_id: String,
    pub public_key: String,
    #[serde(skip_serializing)]
    pub private_key_encrypted: String,
    pub created_at: DateTime<Utc>,
}
