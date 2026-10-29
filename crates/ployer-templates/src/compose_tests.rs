use super::*;

/// A rendered single-service compose, shaped like what `render()` produces for
/// the postgresql template.
const RENDERED: &str = r#"
services:
  postgresql:
    image: postgres:17-alpine
    environment:
      POSTGRES_PASSWORD: hunter2
    volumes:
      - ployer_demo_data:/var/lib/postgresql/data
    networks:
      - ployer
"#;

fn ports_of(yaml: &str) -> Vec<String> {
    parse(yaml)
        .unwrap()
        .services
        .into_values()
        .flat_map(|s| s.ports)
        .collect()
}

// ===== parse_port_spec =====

#[test]
fn a_bare_port_maps_to_itself() {
    assert_eq!(
        parse_port_spec("5432"),
        Some(PortMapping {
            host_ip: None,
            host_port: "5432".into(),
            container_port: "5432".into()
        })
    );
}

#[test]
fn host_and_container_parse_without_an_interface() {
    assert_eq!(
        parse_port_spec("15432:5432"),
        Some(PortMapping {
            host_ip: None,
            host_port: "15432".into(),
            container_port: "5432".into()
        })
    );
}

#[test]
fn a_host_interface_is_kept_not_folded_into_the_host_port() {
    // The bug this guards: splitting on the first ':' read the interface as
    // the host port and "15432:5432" as the container port.
    assert_eq!(
        parse_port_spec("127.0.0.1:15432:5432"),
        Some(PortMapping {
            host_ip: Some("127.0.0.1".into()),
            host_port: "15432".into(),
            container_port: "5432".into()
        })
    );
}

#[test]
fn all_interfaces_parses_like_any_other_interface() {
    assert_eq!(
        parse_port_spec("0.0.0.0:3306:3306"),
        Some(PortMapping {
            host_ip: Some("0.0.0.0".into()),
            host_port: "3306".into(),
            container_port: "3306".into()
        })
    );
}

#[test]
fn a_bracketed_ipv6_interface_parses() {
    assert_eq!(
        parse_port_spec("[::1]:15432:5432"),
        Some(PortMapping {
            host_ip: Some("::1".into()),
            host_port: "15432".into(),
            container_port: "5432".into()
        })
    );
}

#[test]
fn malformed_port_specs_are_rejected() {
    assert_eq!(parse_port_spec(""), None);
    assert_eq!(parse_port_spec(":5432"), None);
    assert_eq!(parse_port_spec("15432:"), None);
    assert_eq!(parse_port_spec("1:2:3:4"), None);
    assert_eq!(parse_port_spec("[::1]5432:5432"), None);
}

#[test]
fn split_port_drops_the_interface() {
    assert_eq!(split_port("5432"), Some(("5432".into(), "5432".into())));
    assert_eq!(split_port("15432:5432"), Some(("15432".into(), "5432".into())));
    assert_eq!(
        split_port("0.0.0.0:15432:5432"),
        Some(("15432".into(), "5432".into()))
    );
}

// ===== add_ports_to_all_services =====

#[test]
fn publishing_writes_the_interface_into_the_compose() {
    let out = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "127.0.0.1").unwrap();
    assert_eq!(ports_of(&out), vec!["127.0.0.1:15432:5432"]);
}

#[test]
fn publishing_on_all_interfaces_is_spelled_out_not_implied() {
    // The stored compose has to record which interface was chosen, so nothing
    // downstream has to guess and so the operator can read it back.
    let out = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "0.0.0.0").unwrap();
    assert_eq!(ports_of(&out), vec!["0.0.0.0:15432:5432"]);
}

#[test]
fn an_empty_interface_falls_back_to_the_two_part_form() {
    let out = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "").unwrap();
    assert_eq!(ports_of(&out), vec!["15432:5432"]);
}

#[test]
fn an_ipv6_interface_is_bracketed_so_it_round_trips() {
    let out = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "::1").unwrap();
    assert_eq!(ports_of(&out), vec!["[::1]:15432:5432"]);
    assert_eq!(
        parse_port_spec(&ports_of(&out)[0]).unwrap().host_ip,
        Some("::1".into())
    );
}

#[test]
fn no_bindings_leaves_the_compose_alone() {
    let out = add_ports_to_all_services(RENDERED, &[], "0.0.0.0").unwrap();
    assert_eq!(out, RENDERED);
}

#[test]
fn republishing_replaces_the_previous_interface() {
    let public = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "0.0.0.0").unwrap();
    let private = add_ports_to_all_services(&public, &[(5432, 15432)], "127.0.0.1").unwrap();
    assert_eq!(ports_of(&private), vec!["127.0.0.1:15432:5432"]);
}

#[test]
fn unpublishing_removes_the_interface_too() {
    let published = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "0.0.0.0").unwrap();
    let unpublished = remove_ports_from_all_services(&published).unwrap();
    assert!(ports_of(&unpublished).is_empty());
}

// ===== current_port_bindings =====

#[test]
fn current_bindings_read_a_three_part_spec() {
    let published = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "127.0.0.1").unwrap();
    assert_eq!(current_port_bindings(&published).unwrap(), vec![(5432, 15432)]);
}

#[test]
fn current_bindings_still_read_a_two_part_spec() {
    let published = add_ports_to_all_services(RENDERED, &[(5432, 15432)], "").unwrap();
    assert_eq!(current_port_bindings(&published).unwrap(), vec![(5432, 15432)]);
}

#[test]
fn current_bindings_are_empty_when_nothing_is_published() {
    assert!(current_port_bindings(RENDERED).unwrap().is_empty());
}

// ===== restart =====

#[test]
fn a_declared_restart_policy_survives_parsing() {
    // It is not enough that the field exists; the deploy path reads it, so a
    // template asking for "no" has to arrive as Some("no").
    let yaml = r#"
services:
  app:
    image: nginx:alpine
    restart: "no"
"#;
    assert_eq!(parse(yaml).unwrap().services["app"].restart.as_deref(), Some("no"));
}

#[test]
fn an_absent_restart_policy_parses_as_none() {
    assert_eq!(parse(RENDERED).unwrap().services["postgresql"].restart, None);
}
