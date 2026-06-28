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
    #[serde(default)]
    pub compose_content: Option<String>,
    #[serde(default)]
    pub template_slug: Option<String>,
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

impl Application {
    /// Docker- and DNS-safe form of the application name.
    ///
    /// Docker repository names and DNS labels only allow lowercase
    /// `[a-z0-9-]`, so derive a slug used for image tags, container names and
    /// subdomains. Without this, a name like `SLW Homes` produces an invalid
    /// reference such as `ployer-SLW Homes`.
    pub fn slug(&self) -> String {
        slugify(&self.name)
    }
}

/// Lowercase, replace any run of non-`[a-z0-9]` with a single `-`, trim
/// leading/trailing `-`. Falls back to `app` if nothing usable remains.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
#[path = "application_tests.rs"]
mod tests;

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
