use super::*;

// ---------------------------------------------------------------------------
// Template parsing
//
// Templates are fetched as YAML from a remote registry that ships
// independently of this binary. Parsing has to be forgiving about what is
// absent and strict about what is required.
// ---------------------------------------------------------------------------

const MINIMAL: &str = r#"
slug: fixture
name: Fixture
description: Fixture service.
category: database
version: 1
compose: |
  services:
    fixture:
      image: fixture:1-alpine
"#;

#[test]
fn a_template_needs_only_identity_and_a_compose_body() {
    let t: Template = serde_yaml::from_str(MINIMAL).expect("minimal template must parse");

    assert_eq!(t.slug, "fixture");
    assert_eq!(t.name, "Fixture");
    assert_eq!(t.category, "database");
    assert!(t.compose.contains("image: fixture:1-alpine"));
}

#[test]
fn optional_sections_default_to_empty_rather_than_failing() {
    let t: Template = serde_yaml::from_str(MINIMAL).unwrap();

    assert!(t.icon.is_none());
    assert!(t.tags.is_empty());
    assert!(t.inputs.is_empty());
    assert!(t.ports.is_empty());
    assert!(t.volumes.is_empty());
    assert!(t.post_install.is_none());
}

#[test]
fn a_template_without_a_compose_body_is_rejected() {
    let yaml = "slug: fixture\nname: Fixture\ndescription: d\ncategory: database\nversion: 1\n";
    assert!(
        serde_yaml::from_str::<Template>(yaml).is_err(),
        "there is nothing to deploy without a compose body"
    );
}

#[test]
fn a_template_without_a_slug_is_rejected() {
    let yaml = "name: Fixture\ndescription: d\ncategory: database\nversion: 1\ncompose: \"x\"\n";
    assert!(serde_yaml::from_str::<Template>(yaml).is_err());
}

#[test]
fn unrecognised_top_level_keys_are_ignored() {
    // The registry can ship a newer schema than this binary understands;
    // an older Ployer must keep installing rather than hard-fail.
    let yaml = format!("{MINIMAL}\nfuture_field: whatever\n");
    let t: Template = serde_yaml::from_str(&yaml).expect("unknown keys must not break parsing");
    assert_eq!(t.slug, "fixture");
}

#[test]
fn version_accepts_both_a_number_and_a_string() {
    for raw in ["version: 1", "version: \"1.2.0\""] {
        let yaml = MINIMAL.replace("version: 1", raw);
        let t: Template = serde_yaml::from_str(&yaml).unwrap_or_else(|e| panic!("{raw}: {e}"));
        assert!(!t.version.is_null());
    }
}

// ---------------------------------------------------------------------------
// Input declarations
// ---------------------------------------------------------------------------

#[test]
fn input_type_names_map_to_their_kinds() {
    for (raw, expected) in [
        ("string", InputKind::String),
        ("password", InputKind::Password),
        ("number", InputKind::Number),
        ("bool", InputKind::Bool),
    ] {
        let yaml = format!("key: K\nlabel: L\ntype: {raw}\n");
        let input: Input = serde_yaml::from_str(&yaml).unwrap_or_else(|e| panic!("{raw}: {e}"));
        assert_eq!(input.kind, expected);
    }
}

#[test]
fn an_unknown_input_type_is_rejected() {
    let yaml = "key: K\nlabel: L\ntype: secret\n";
    assert!(
        serde_yaml::from_str::<Input>(yaml).is_err(),
        "an unsupported input type must not be read as something else"
    );
}

#[test]
fn generator_kinds_map_to_password_and_hex() {
    for (raw, expected) in [
        ("password", GenerateKind::Password),
        ("hex", GenerateKind::Hex),
    ] {
        let yaml = format!("kind: {raw}\nlength: 16\n");
        let g: Generate = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(g.kind, expected);
        assert_eq!(g.length, Some(16));
    }
}

#[test]
fn a_generator_without_a_length_parses_and_leaves_the_default_to_the_renderer() {
    let g: Generate = serde_yaml::from_str("kind: password\n").unwrap();
    assert!(g.length.is_none());
}

// ---------------------------------------------------------------------------
// Ports and volumes
// ---------------------------------------------------------------------------

#[test]
fn a_port_is_not_exposed_unless_it_says_so() {
    let p: Port = serde_yaml::from_str("container: 5432\n").unwrap();
    assert_eq!(p.container, 5432);
    assert!(
        !p.expose,
        "defaulting to exposed would publish databases to the host by accident"
    );
}

#[test]
fn a_port_number_above_the_u16_range_is_rejected() {
    assert!(serde_yaml::from_str::<Port>("container: 70000\n").is_err());
}

#[test]
fn a_volume_declares_a_name_and_a_mount_path() {
    let v: Volume = serde_yaml::from_str("name: data\nmount: /var/lib/fixture/data\n").unwrap();
    assert_eq!(v.name, "data");
    assert_eq!(v.mount, "/var/lib/fixture/data");
}

// ---------------------------------------------------------------------------
// Registry index
//
// `GET /api/v1/templates` is served from this structure, and the dashboard
// reads these exact field names.
// ---------------------------------------------------------------------------

const INDEX: &str = r#"{
  "schema_version": 1,
  "generated_at": "2026-10-19T10:03:38.550Z",
  "templates": [
    {
      "slug": "fixture",
      "name": "Fixture",
      "description": "Fixture service.",
      "category": "database",
      "icon": "fixture.svg",
      "tags": ["database", "sql"],
      "version": 1,
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "url": "https://ployer.nusendra.com/templates/fixture.yaml"
    }
  ]
}"#;

#[test]
fn registry_index_parses_into_entries() {
    let index: RegistryIndex = serde_json::from_str(INDEX).expect("index must parse");

    assert_eq!(index.schema_version, 1);
    assert_eq!(index.templates.len(), 1);
    assert_eq!(index.templates[0].slug, "fixture");
    assert_eq!(index.templates[0].category, "database");
    assert_eq!(index.templates[0].tags, vec!["database", "sql"]);
}

#[test]
fn an_index_entry_must_carry_a_sha256_and_url() {
    // Without these the backend cannot fetch or verify the template, so a
    // missing field has to be a parse error rather than an empty default.
    for field in ["sha256", "url"] {
        let stripped: serde_json::Value = {
            let mut v: serde_json::Value = serde_json::from_str(INDEX).unwrap();
            v["templates"][0]
                .as_object_mut()
                .unwrap()
                .remove(field)
                .unwrap();
            v
        };
        assert!(
            serde_json::from_value::<RegistryIndex>(stripped).is_err(),
            "an entry without `{field}` must be rejected"
        );
    }
}

#[test]
fn an_index_entry_without_an_icon_or_tags_still_parses() {
    let json = r#"{
      "schema_version": 1,
      "generated_at": "2026-10-19T10:03:38.550Z",
      "templates": [{
        "slug": "fixture", "name": "Fixture", "description": "d",
        "category": "database", "version": 1,
        "sha256": "00", "url": "https://example.test/fixture.yaml"
      }]
    }"#;
    let index: RegistryIndex = serde_json::from_str(json).unwrap();
    assert!(index.templates[0].icon.is_none());
    assert!(index.templates[0].tags.is_empty());
}

// ---------------------------------------------------------------------------
// API serialisation
//
// `GET /api/v1/templates/:slug` returns `Template` verbatim, so the JSON key
// names here are a published contract the dashboard depends on.
// ---------------------------------------------------------------------------

#[test]
fn template_serialises_with_the_field_names_the_dashboard_reads() {
    let t: Template = serde_yaml::from_str(MINIMAL).unwrap();
    let json = serde_json::to_value(&t).unwrap();
    let obj = json.as_object().expect("template serialises to an object");

    for key in [
        "slug",
        "name",
        "description",
        "category",
        "icon",
        "tags",
        "version",
        "inputs",
        "ports",
        "volumes",
        "compose",
        "post_install",
    ] {
        assert!(obj.contains_key(key), "missing `{key}` in the API payload");
    }
}

#[test]
fn an_input_serialises_its_type_under_the_key_type() {
    // `kind` is renamed to `type` on the wire; the install form keys off it.
    let input: Input = serde_yaml::from_str("key: K\nlabel: L\ntype: password\n").unwrap();
    let json = serde_json::to_value(&input).unwrap();

    assert_eq!(json["type"], "password");
    assert!(
        json.get("kind").is_none(),
        "`kind` must not leak into the API payload"
    );
}

#[test]
fn a_template_survives_a_json_round_trip() {
    let yaml = format!(
        "{MINIMAL}\ninputs:\n  - key: K\n    label: L\n    type: password\n    generate:\n      kind: password\n      length: 32\nports:\n  - container: 5432\nvolumes:\n  - name: data\n    mount: /var/lib/fixture/data\n"
    );
    let original: Template = serde_yaml::from_str(&yaml).unwrap();
    let round_tripped: Template =
        serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();

    assert_eq!(round_tripped.slug, original.slug);
    assert_eq!(round_tripped.inputs.len(), 1);
    assert_eq!(round_tripped.inputs[0].kind, InputKind::Password);
    assert_eq!(round_tripped.inputs[0].generate.as_ref().unwrap().length, Some(32));
    assert_eq!(round_tripped.ports[0].container, 5432);
    assert_eq!(round_tripped.volumes[0].mount, "/var/lib/fixture/data");
}

#[test]
fn post_install_outputs_keep_their_label_and_value() {
    let yaml = format!(
        "{MINIMAL}\npost_install:\n  message: hello\n  outputs:\n    - label: Internal connection URI\n      value: \"proto://user@host:5432/db\"\n"
    );
    let t: Template = serde_yaml::from_str(&yaml).unwrap();
    let pi = t.post_install.expect("post_install present");

    assert_eq!(pi.message.as_deref(), Some("hello"));
    assert_eq!(pi.outputs.len(), 1);
    assert_eq!(pi.outputs[0].label, "Internal connection URI");
    assert_eq!(pi.outputs[0].value, "proto://user@host:5432/db");
}

// ---------------------------------------------------------------------------
// Cross-repo compatibility: the removed `icon` key
//
// `icon` was dropped from `Template` and `IndexEntry` in ac178ed, but
// `ployer` (this binary) and `ployer-website` (which generates and serves the
// registry) deploy independently. The published catalog still ships the key on
// every entry, so the parser has to keep ignoring it.
//
// Verified against the live registry on 2026-10-29:
//
//   $ curl -s https://ployer.nusendra.com/templates/index.json | grep -c '"icon"'
//   4                                   # jellyfin, mariadb, mongodb, postgresql
//   $ curl -s https://ployer.nusendra.com/templates/postgresql.yaml | grep icon
//   icon: postgresql.svg
//
// These two tests exist to fail loudly if anyone adds
// `#[serde(deny_unknown_fields)]` to either struct. That attribute looks like
// tightening; here it would make every install fail on a field nobody reads.
// ---------------------------------------------------------------------------

/// The index is fetched before anything else, so a strict parser here breaks
/// the catalog listing outright — no template is installable.
#[test]
fn an_index_entry_carrying_the_removed_icon_key_still_parses() {
    // Shape copied from the live index.json, trimmed to one entry.
    let json = r#"{
      "schema_version": 1,
      "generated_at": "2026-10-29T04:56:52.749Z",
      "templates": [{
        "slug": "postgresql",
        "name": "PostgreSQL",
        "description": "Object-relational database with strong SQL standards compliance.",
        "category": "database",
        "icon": "postgresql.svg",
        "tags": ["database", "sql", "postgres"],
        "version": 1,
        "sha256": "00",
        "url": "https://ployer.nusendra.com/templates/postgresql.yaml"
      }]
    }"#;

    let index: RegistryIndex =
        serde_json::from_str(json).expect("a stale `icon` key must not break index parsing");

    assert_eq!(index.templates[0].slug, "postgresql");
    assert_eq!(index.templates[0].tags, ["database", "sql", "postgres"]);
}

/// Same guarantee one level down: the template YAML served from the registry
/// still declares `icon:`, so `Template` must ignore it too.
#[test]
fn a_template_carrying_the_removed_icon_key_still_parses() {
    // Shape copied from the live postgresql.yaml, trimmed to the required keys.
    let yaml = r#"
slug: postgresql
name: PostgreSQL
description: Object-relational database with strong SQL standards compliance.
category: database
icon: postgresql.svg
tags: [database, sql, postgres]
version: 1
compose: |
  services:
    postgresql:
      image: postgres:17-alpine
"#;

    let t: Template =
        serde_yaml::from_str(yaml).expect("a stale `icon:` must not break template parsing");

    assert_eq!(t.slug, "postgresql");
    assert!(t.compose.contains("image: postgres:17-alpine"));
}
