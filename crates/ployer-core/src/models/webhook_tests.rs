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
