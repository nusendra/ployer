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
