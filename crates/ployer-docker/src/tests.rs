use super::*;

fn make_config(cpu: Option<f64>, mem_mb: Option<i64>) -> ContainerConfig {
    ContainerConfig {
        image: "myapp:latest".to_string(),
        name: Some("test-container".to_string()),
        env: None,
        ports: None,
        volumes: None,
        network: None,
        cmd: None,
        cpu_limit: cpu,
        memory_limit: mem_mb,
    }
}

#[test]
fn cpu_limit_converts_to_nano_cpus() {
    // 0.5 cores = 500_000_000 nano_cpus
    let config = make_config(Some(0.5), None);
    let nano_cpus = config.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64);
    assert_eq!(nano_cpus, Some(500_000_000));
}

#[test]
fn cpu_limit_one_core_is_one_billion_nano_cpus() {
    let config = make_config(Some(1.0), None);
    let nano_cpus = config.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64);
    assert_eq!(nano_cpus, Some(1_000_000_000));
}

#[test]
fn memory_limit_converts_mb_to_bytes() {
    // 512 MB = 536_870_912 bytes
    let config = make_config(None, Some(512));
    let bytes = config.memory_limit.map(|m| m * 1024 * 1024);
    assert_eq!(bytes, Some(512 * 1024 * 1024));
}

#[test]
fn memory_limit_1gb() {
    let config = make_config(None, Some(1024));
    let bytes = config.memory_limit.map(|m| m * 1024 * 1024);
    assert_eq!(bytes, Some(1024 * 1024 * 1024));
}

#[test]
fn no_limits_produce_none() {
    let config = make_config(None, None);
    let nano_cpus = config.cpu_limit.map(|c| (c * 1_000_000_000.0) as i64);
    let memory = config.memory_limit.map(|m| m * 1024 * 1024);
    assert_eq!(nano_cpus, None);
    assert_eq!(memory, None);
}

#[test]
fn container_config_fields_are_set() {
    let config = make_config(Some(2.0), Some(256));
    assert_eq!(config.image, "myapp:latest");
    assert_eq!(config.name, Some("test-container".to_string()));
    assert_eq!(config.cpu_limit, Some(2.0));
    assert_eq!(config.memory_limit, Some(256));
}
