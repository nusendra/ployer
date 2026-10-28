use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub application_id: String,
    pub provider: WebhookProvider,
    pub secret: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebhookProvider {
    GitHub,
    GitLab,
}

impl WebhookProvider {
    pub fn as_str(&self) -> &str {
        match self {
            WebhookProvider::GitHub => "github",
            WebhookProvider::GitLab => "gitlab",
        }
    }

    /// Parse a `WebhookProvider` out of the string stored in its database column.
    ///
    /// Total and lossy by design: an unrecognised value yields
    /// [`WebhookProvider::GitHub`] rather than an error, so a row written by a newer
    /// schema can never make an existing row unreadable.
    ///
    /// Matching is case-insensitive.
    ///
    /// Deliberately not [`std::str::FromStr`]: that trait would need
    /// `Err = Infallible`, which hides the fallback from callers and makes
    /// `"nonsense".parse()` look like a successful parse.
    pub fn from_db_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "github" => WebhookProvider::GitHub,
            "gitlab" => WebhookProvider::GitLab,
            _ => WebhookProvider::GitHub,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub webhook_id: String,
    pub application_id: String,
    pub provider: WebhookProvider,
    pub event_type: String,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub author: Option<String>,
    pub status: WebhookDeliveryStatus,
    pub response_code: Option<i32>,
    pub error_message: Option<String>,
    pub deployment_id: Option<String>,
    pub delivered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebhookDeliveryStatus {
    Success,
    Failed,
    Skipped,
}

impl WebhookDeliveryStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WebhookDeliveryStatus::Success => "success",
            WebhookDeliveryStatus::Failed => "failed",
            WebhookDeliveryStatus::Skipped => "skipped",
        }
    }

    /// Parse a `WebhookDeliveryStatus` out of the string stored in its database column.
    ///
    /// Total and lossy by design: an unrecognised value yields
    /// [`WebhookDeliveryStatus::Failed`] rather than an error, so a row written by a newer
    /// schema can never make an existing row unreadable.
    ///
    /// Deliberately not [`std::str::FromStr`]: that trait would need
    /// `Err = Infallible`, which hides the fallback from callers and makes
    /// `"nonsense".parse()` look like a successful parse.
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "success" => WebhookDeliveryStatus::Success,
            "failed" => WebhookDeliveryStatus::Failed,
            "skipped" => WebhookDeliveryStatus::Skipped,
            _ => WebhookDeliveryStatus::Failed,
        }
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod tests;
