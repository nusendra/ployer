use super::*;

/// A compose document shaped like the one a database template renders to:
/// one service, a quoted environment map, a named volume, an external network.
const RENDERED: &str = r#"
services:
  fixture:
    image: fixture:1-alpine
    restart: unless-stopped
    environment:
      DB_USER: "app"
      DB_PASSWORD: "s3cret"
      DB_NAME: "app"
    volumes:
      - ployer_shop_data:/var/lib/fixture/data
    networks: [ployer]
networks:
  ployer:
    external: true
"#;

fn single_service(yaml: &str) -> ServiceSpec {
    let spec = parse(yaml).expect("compose must parse");
    assert_eq!(spec.services.len(), 1, "fixture declares one service");
    spec.services.values().next().unwrap().clone()
}

// ---------------------------------------------------------------------------
// The six-key subset the backend actually understands
// ---------------------------------------------------------------------------

#[test]
fn rendered_compose_parses_into_the_supported_subset() {
    let svc = single_service(RENDERED);

    assert_eq!(svc.image, "fixture:1-alpine");
    assert_eq!(svc.restart.as_deref(), Some("unless-stopped"));
    assert_eq!(svc.volumes, vec!["ployer_shop_data:/var/lib/fixture/data"]);
    assert_eq!(svc.networks, vec!["ployer"]);
    assert!(svc.ports.is_empty(), "nothing is published to the host by default");
}

#[test]
fn environment_map_becomes_key_equals_value_pairs() {
    let svc = single_service(RENDERED);
    let pairs = svc.environment.expect("environment present").to_pairs();

    // BTreeMap ordering, so this is deterministic rather than incidental.
    assert_eq!(
        pairs,
        vec![
            "DB_NAME=app".to_string(),
            "DB_PASSWORD=s3cret".to_string(),
            "DB_USER=app".to_string(),
        ]
    );
}

#[test]
fn environment_list_form_is_passed_through_unchanged() {
    let yaml = r#"
services:
  fixture:
    image: fixture:1-alpine
    environment:
      - DB_USER=app
      - DB_PASSWORD=s3cret
"#;
    let svc = single_service(yaml);
    assert_eq!(
        svc.environment.unwrap().to_pairs(),
        vec!["DB_USER=app".to_string(), "DB_PASSWORD=s3cret".to_string()]
    );
}

#[test]
fn service_without_an_environment_block_is_valid() {
    let yaml = "services:\n  fixture:\n    image: fixture:1-alpine\n";
    let svc = single_service(yaml);
    assert!(svc.environment.is_none());
}

#[test]
fn service_without_an_image_is_rejected() {
    // `image` is the one field the Docker call cannot invent a default for.
    let yaml = "services:\n  fixture:\n    restart: always\n";
    assert!(parse(yaml).is_err(), "a service with no image must not parse");
}

// ---------------------------------------------------------------------------
// Non-scalar environment values
// ---------------------------------------------------------------------------

#[test]
fn numeric_and_boolean_environment_values_are_stringified() {
    let yaml = r#"
services:
  fixture:
    image: fixture:1-alpine
    environment:
      PORT: 5432
      DEBUG: true
      EMPTY:
"#;
    let pairs = single_service(yaml).environment.unwrap().to_pairs();
    assert!(pairs.contains(&"PORT=5432".to_string()));
    assert!(pairs.contains(&"DEBUG=true".to_string()));
    assert!(pairs.contains(&"EMPTY=".to_string()));
}

// ---------------------------------------------------------------------------
// Why template authors must quote interpolated environment values.
//
// Input values reach the compose body by raw text substitution, so an
// unquoted placeholder lets a user-supplied database name change the YAML
// structure. These two tests are a matched pair: the first shows the break,
// the second shows quoting fixing it.
// ---------------------------------------------------------------------------

#[test]
fn unquoted_environment_value_containing_a_colon_breaks_the_parse() {
    let yaml = "services:\n  fixture:\n    image: fixture:1-alpine\n    environment:\n      DB_NAME: my: db\n";
    assert!(
        parse(yaml).is_err(),
        "an unquoted colon-space must be a parse error, not a silently odd value"
    );
}

#[test]
fn quoted_environment_value_containing_a_colon_survives_intact() {
    let yaml = "services:\n  fixture:\n    image: fixture:1-alpine\n    environment:\n      DB_NAME: \"my: db\"\n";
    let pairs = single_service(yaml).environment.unwrap().to_pairs();
    assert_eq!(pairs, vec!["DB_NAME=my: db".to_string()]);
}

#[test]
fn quoted_leading_zero_value_keeps_its_zero() {
    // Unquoted, "08" is the kind of value YAML implementations disagree about.
    let yaml = "services:\n  fixture:\n    image: fixture:1-alpine\n    environment:\n      CODE: \"08\"\n";
    let pairs = single_service(yaml).environment.unwrap().to_pairs();
    assert_eq!(pairs, vec!["CODE=08".to_string()]);
}

// ---------------------------------------------------------------------------
// Keys outside the subset are DROPPED, not rejected.
//
// This is a sharp edge: a template author can add `healthcheck:` and it will
// parse cleanly while doing nothing at all. These tests exist so that
// behaviour is discovered here rather than in production.
// ---------------------------------------------------------------------------

#[test]
fn healthcheck_is_silently_ignored_rather_than_honoured() {
    let yaml = r#"
services:
  fixture:
    image: fixture:1-alpine
    healthcheck:
      test: ["CMD-SHELL", "pg_isready"]
      interval: 10s
"#;
    let svc = single_service(yaml);
    assert_eq!(svc.image, "fixture:1-alpine");
    // No field on ServiceSpec can carry it. A template relying on a
    // healthcheck would never become healthy; containers only ever report
    // "Up". If this crate ever grows healthcheck support, this test should
    // fail and be rewritten.
    assert_eq!(
        std::mem::size_of_val(&svc.restart),
        std::mem::size_of::<Option<String>>()
    );
}

#[test]
fn depends_on_and_command_are_silently_ignored() {
    let yaml = r#"
services:
  fixture:
    image: fixture:1-alpine
    command: ["postgres", "-c", "max_connections=200"]
    depends_on:
      - other
"#;
    let svc = single_service(yaml);
    assert_eq!(svc.image, "fixture:1-alpine");
    assert!(svc.volumes.is_empty());
}

// ---------------------------------------------------------------------------
// Host port publishing — the "Expose to host" checkbox
// ---------------------------------------------------------------------------

#[test]
fn adding_a_binding_publishes_host_colon_container() {
    let out = add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap();
    let svc = single_service(&out);
    assert_eq!(svc.ports, vec!["15432:5432"]);
}

#[test]
fn adding_no_bindings_returns_the_document_untouched() {
    let out = add_ports_to_all_services(RENDERED, &[]).unwrap();
    assert_eq!(out, RENDERED, "an empty binding list must be a no-op");
}

#[test]
fn removing_ports_clears_a_previously_published_binding() {
    let exposed = add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap();
    let unexposed = remove_ports_from_all_services(&exposed).unwrap();

    assert!(single_service(&unexposed).ports.is_empty());
}

#[test]
fn unexposing_preserves_image_environment_and_volumes() {
    // Un-exposing must not take the database's data or credentials with it.
    let exposed = add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap();
    let unexposed = remove_ports_from_all_services(&exposed).unwrap();
    let svc = single_service(&unexposed);

    assert_eq!(svc.image, "fixture:1-alpine");
    assert_eq!(svc.volumes, vec!["ployer_shop_data:/var/lib/fixture/data"]);
    assert_eq!(svc.networks, vec!["ployer"]);
    assert!(svc
        .environment
        .unwrap()
        .to_pairs()
        .contains(&"DB_PASSWORD=s3cret".to_string()));
}

#[test]
fn expose_then_unexpose_round_trips_to_the_same_parsed_spec() {
    let once = remove_ports_from_all_services(
        &add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap(),
    )
    .unwrap();
    let twice = remove_ports_from_all_services(
        &add_ports_to_all_services(&once, &[(5432, 15432)]).unwrap(),
    )
    .unwrap();

    assert_eq!(once, twice, "repeated expose/unexpose must converge");
}

#[test]
fn re_exposing_replaces_the_binding_instead_of_appending() {
    let first = add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap();
    let second = add_ports_to_all_services(&first, &[(5432, 25432)]).unwrap();

    assert_eq!(
        single_service(&second).ports,
        vec!["25432:5432"],
        "changing the host port must not leave the old one behind"
    );
}

#[test]
fn current_port_bindings_reports_container_then_host() {
    let exposed = add_ports_to_all_services(RENDERED, &[(5432, 15432)]).unwrap();
    assert_eq!(current_port_bindings(&exposed).unwrap(), vec![(5432, 15432)]);
}

#[test]
fn current_port_bindings_is_empty_when_nothing_is_published() {
    assert!(current_port_bindings(RENDERED).unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Spec splitting helpers
// ---------------------------------------------------------------------------

#[test]
fn split_volume_separates_source_from_mount_path() {
    assert_eq!(
        split_volume("ployer_shop_data:/var/lib/fixture/data"),
        Some(("ployer_shop_data".into(), "/var/lib/fixture/data".into()))
    );
}

#[test]
fn split_volume_ignores_a_trailing_access_mode() {
    assert_eq!(
        split_volume("ployer_shop_data:/var/lib/fixture/data:ro"),
        Some(("ployer_shop_data".into(), "/var/lib/fixture/data".into()))
    );
}

#[test]
fn split_volume_returns_none_without_a_mount_path() {
    assert_eq!(split_volume("ployer_shop_data"), None);
}

#[test]
fn split_port_maps_a_bare_port_to_itself() {
    assert_eq!(split_port("5432"), Some(("5432".into(), "5432".into())));
}

#[test]
fn split_port_separates_host_from_container() {
    assert_eq!(split_port("15432:5432"), Some(("15432".into(), "5432".into())));
}
