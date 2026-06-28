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

#[test]
fn slugify_makes_docker_and_dns_safe_names() {
    assert_eq!(slugify("SLW Homes"), "slw-homes");
    assert_eq!(slugify("My_App.v2"), "my-app-v2");
    assert_eq!(slugify("  Leading/Trailing  "), "leading-trailing");
    assert_eq!(slugify("already-slug"), "already-slug");
    assert_eq!(slugify("Café!!"), "caf");
    assert_eq!(slugify("***"), "app");
    assert_eq!(slugify(""), "app");
}
