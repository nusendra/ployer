use super::*;

#[test]
fn deployment_status_round_trips() {
    for (variant, s) in [
        (DeploymentStatus::Queued, "queued"),
        (DeploymentStatus::Cloning, "cloning"),
        (DeploymentStatus::Building, "building"),
        (DeploymentStatus::Deploying, "deploying"),
        (DeploymentStatus::Running, "running"),
        (DeploymentStatus::Failed, "failed"),
        (DeploymentStatus::Cancelled, "cancelled"),
        (DeploymentStatus::RolledBack, "rolled_back"),
    ] {
        assert_eq!(variant.as_str(), s);
        assert_eq!(DeploymentStatus::from_str(s), variant);
    }
}

#[test]
fn deployment_status_unknown_defaults_to_queued() {
    assert_eq!(DeploymentStatus::from_str("unknown"), DeploymentStatus::Queued);
}

#[test]
fn health_check_status_round_trips() {
    for (variant, s) in [
        (HealthCheckStatus::Healthy, "healthy"),
        (HealthCheckStatus::Unhealthy, "unhealthy"),
        (HealthCheckStatus::Unknown, "unknown"),
    ] {
        assert_eq!(variant.as_str(), s);
        assert_eq!(HealthCheckStatus::from_str(s), variant);
    }
}

#[test]
fn health_check_status_unknown_defaults_to_unknown() {
    assert_eq!(HealthCheckStatus::from_str("bad"), HealthCheckStatus::Unknown);
}
