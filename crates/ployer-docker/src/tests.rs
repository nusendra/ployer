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
        restart: None,
    }
}

fn published(ports: &[(&str, PublishedPort)]) -> HashMap<String, PublishedPort> {
    ports.iter().map(|(p, b)| (p.to_string(), b.clone())).collect()
}

/// The HostIp Docker would be handed for `container_port`.
fn bound_host_ip(bindings: &PortBindings, container_port: &str) -> String {
    bindings
        .get(container_port)
        .and_then(|b| b.as_ref())
        .and_then(|b| b.first())
        .and_then(|b| b.host_ip.clone())
        .unwrap_or_else(|| panic!("no binding for {container_port}"))
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

// ===== Bind address =====
//
// These pin the host interface a published port lands on. The whole point of
// the type is that nobody reaches a database over the LAN by accident, so the
// default and the explicit opt-in both get asserted.

#[test]
fn bind_address_defaults_to_loopback() {
    assert_eq!(BindAddress::default(), BindAddress::Loopback);
    assert_eq!(BindAddress::default().host_ip(), "127.0.0.1");
}

#[test]
fn bind_address_maps_variants_to_host_ips() {
    assert_eq!(BindAddress::Loopback.host_ip(), "127.0.0.1");
    assert_eq!(BindAddress::AllInterfaces.host_ip(), "0.0.0.0");
    assert_eq!(BindAddress::Address("100.92.195.33".into()).host_ip(), "100.92.195.33");
}

#[test]
fn an_unspecified_host_ip_resolves_to_loopback_not_all_interfaces() {
    // Compose files written before this existed carry "15432:5432" with no
    // interface. They must not keep landing on 0.0.0.0 by omission.
    assert_eq!(BindAddress::from_host_ip(None), BindAddress::Loopback);
    assert_eq!(BindAddress::from_host_ip(Some("")), BindAddress::Loopback);
    assert_eq!(BindAddress::from_host_ip(Some("  ")), BindAddress::Loopback);
}

#[test]
fn from_host_ip_round_trips_the_named_variants() {
    assert_eq!(BindAddress::from_host_ip(Some("127.0.0.1")), BindAddress::Loopback);
    assert_eq!(BindAddress::from_host_ip(Some("0.0.0.0")), BindAddress::AllInterfaces);
    assert_eq!(
        BindAddress::from_host_ip(Some("192.168.100.100")),
        BindAddress::Address("192.168.100.100".into())
    );
}

#[test]
fn only_loopback_addresses_count_as_host_only() {
    assert!(BindAddress::Loopback.is_loopback_only());
    assert!(BindAddress::Address("127.0.0.53".into()).is_loopback_only());
    assert!(BindAddress::Address("::1".into()).is_loopback_only());
    assert!(!BindAddress::AllInterfaces.is_loopback_only());
    assert!(!BindAddress::Address("192.168.100.100".into()).is_loopback_only());
    // A tailnet address is still another machine's route in.
    assert!(!BindAddress::Address("100.92.195.33".into()).is_loopback_only());
}

#[test]
fn published_ports_bind_where_the_config_says() {
    let (_, bindings) = build_port_bindings(&published(&[
        ("5432/tcp", PublishedPort::new(BindAddress::Loopback, "15432")),
        ("3306/tcp", PublishedPort::new(BindAddress::AllInterfaces, "3306")),
        ("8096/tcp", PublishedPort::new(BindAddress::Address("100.92.195.33".into()), "8096")),
    ]));

    assert_eq!(bound_host_ip(&bindings, "5432/tcp"), "127.0.0.1");
    assert_eq!(bound_host_ip(&bindings, "3306/tcp"), "0.0.0.0");
    assert_eq!(bound_host_ip(&bindings, "8096/tcp"), "100.92.195.33");
}

#[test]
fn an_ephemeral_port_is_loopback_bound_with_an_empty_host_port() {
    // The app deploy path relies on the empty host port for Docker's ephemeral
    // assignment, and Caddy reaches it over localhost — so it never needs to
    // leave the host.
    let publish = PublishedPort::ephemeral();
    assert_eq!(publish.bind, BindAddress::Loopback);
    assert_eq!(publish.host_port, "");

    let (exposed, bindings) = build_port_bindings(&published(&[("3000/tcp", publish)]));
    assert_eq!(bound_host_ip(&bindings, "3000/tcp"), "127.0.0.1");
    assert_eq!(
        bindings["3000/tcp"].as_ref().unwrap()[0].host_port,
        Some(String::new())
    );
    assert!(exposed.contains_key("3000/tcp"));
}

#[test]
fn every_published_port_is_also_exposed() {
    let (exposed, bindings) = build_port_bindings(&published(&[
        ("5432/tcp", PublishedPort::new(BindAddress::Loopback, "15432")),
        ("5433/tcp", PublishedPort::new(BindAddress::Loopback, "15433")),
    ]));
    assert_eq!(exposed.len(), 2);
    assert_eq!(bindings.len(), 2);
}

#[test]
fn bind_address_serde_uses_snake_case_tags() {
    assert_eq!(serde_json::to_string(&BindAddress::Loopback).unwrap(), "\"loopback\"");
    assert_eq!(
        serde_json::to_string(&BindAddress::AllInterfaces).unwrap(),
        "\"all_interfaces\""
    );
    assert_eq!(
        serde_json::from_str::<BindAddress>("\"all_interfaces\"").unwrap(),
        BindAddress::AllInterfaces
    );
    assert_eq!(
        serde_json::from_str::<BindAddress>("{\"address\":\"100.92.195.33\"}").unwrap(),
        BindAddress::Address("100.92.195.33".into())
    );
}

#[test]
fn a_published_port_without_a_bind_field_deserializes_to_loopback() {
    let publish: PublishedPort = serde_json::from_str("{\"host_port\":\"15432\"}").unwrap();
    assert_eq!(publish.bind, BindAddress::Loopback);
}

// ===== Restart policy =====

#[test]
fn an_unnamed_restart_policy_defaults_to_unless_stopped() {
    let spec = parse_restart_policy(None).unwrap();
    assert_eq!(spec.name, RestartPolicyNameEnum::UNLESS_STOPPED);
    assert_eq!(spec.max_retries, None);
    // An empty string is the same "no opinion".
    assert_eq!(parse_restart_policy(Some("")).unwrap().name, RestartPolicyNameEnum::UNLESS_STOPPED);
}

#[test]
fn a_declared_restart_policy_is_honoured_not_overridden() {
    // A template asking for `restart: "no"` used to silently get
    // unless-stopped. It gets what it asked for.
    assert_eq!(parse_restart_policy(Some("no")).unwrap().name, RestartPolicyNameEnum::NO);
    assert_eq!(parse_restart_policy(Some("always")).unwrap().name, RestartPolicyNameEnum::ALWAYS);
    assert_eq!(
        parse_restart_policy(Some("on-failure")).unwrap().name,
        RestartPolicyNameEnum::ON_FAILURE
    );
    assert_eq!(
        parse_restart_policy(Some("unless-stopped")).unwrap().name,
        RestartPolicyNameEnum::UNLESS_STOPPED
    );
}

#[test]
fn on_failure_carries_its_retry_cap() {
    let spec = parse_restart_policy(Some("on-failure:5")).unwrap();
    assert_eq!(spec.name, RestartPolicyNameEnum::ON_FAILURE);
    assert_eq!(spec.max_retries, Some(5));
}

#[test]
fn an_unknown_restart_policy_is_an_error_not_a_silent_fallback() {
    let err = parse_restart_policy(Some("nope")).unwrap_err().to_string();
    assert!(err.contains("nope"), "{err}");
    assert!(parse_restart_policy(Some("always:3")).is_err());
    assert!(parse_restart_policy(Some("on-failure:many")).is_err());
}
