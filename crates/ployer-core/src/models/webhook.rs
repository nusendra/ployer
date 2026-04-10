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

    pub fn from_str(s: &str) -> Self {
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

    pub fn from_str(s: &str) -> Self {
        match s {
            "success" => WebhookDeliveryStatus::Success,
            "failed" => WebhookDeliveryStatus::Failed,
            "skipped" => WebhookDeliveryStatus::Skipped,
            _ => WebhookDeliveryStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_provider_round_trips() {
        assert_eq!(WebhookProvider::GitHub.as_str(), "github");
        assert_eq!(WebhookProvider::GitLab.as_str(), "gitlab");
        assert_eq!(WebhookProvider::from_str("github"), WebhookProvider::GitHub);
        assert_eq!(WebhookProvider::from_str("gitlab"), WebhookProvider::GitLab);
    }

    #[test]
    fn webhook_provider_is_case_insensitive() {
        assert_eq!(WebhookProvider::from_str("GitHub"), WebhookProvider::GitHub);
        assert_eq!(WebhookProvider::from_str("GitLab"), WebhookProvider::GitLab);
    }

    #[test]
    fn webhook_provider_unknown_defaults_to_github() {
        assert_eq!(WebhookProvider::from_str("bitbucket"), WebhookProvider::GitHub);
    }

    #[test]
    fn webhook_delivery_status_round_trips() {
        for (variant, s) in [
            (WebhookDeliveryStatus::Success, "success"),
            (WebhookDeliveryStatus::Failed, "failed"),
            (WebhookDeliveryStatus::Skipped, "skipped"),
        ] {
            assert_eq!(variant.as_str(), s);
            assert_eq!(WebhookDeliveryStatus::from_str(s), variant);
        }
    }

    #[test]
    fn webhook_delivery_status_unknown_defaults_to_failed() {
        assert_eq!(WebhookDeliveryStatus::from_str("unknown"), WebhookDeliveryStatus::Failed);
    }
}
