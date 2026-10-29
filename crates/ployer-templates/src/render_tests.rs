use chrono::Utc;
use ployer_core::models::{AppStatus, Application, BuildStrategy};

use super::*;
use crate::schema::{OutputItem, PostInstall, Template, Volume};

/// A service template shaped exactly like the real database templates in the
/// registry (generated password, named volume, external network, credential
/// URI in `post_install`), but with fixture-only identifiers.
///
/// Deliberately NOT a copy of `postgresql.yaml`: the registry lives in a
/// different repo (`ployer-website/static/templates`) and is the single source
/// of truth for image tags, ports and mount paths. Those are asserted there.
/// These tests own the *rendering engine* contract that every template relies
/// on, so they must not silently drift into asserting catalog content.
const FIXTURE: &str = r#"
slug: fixture-db
name: Fixture DB
description: Fixture relational database.
category: database
tags: [database, sql]
version: 1

inputs:
  - key: DB_USER
    label: Database user
    type: string
    default: app
  - key: DB_PASSWORD
    label: Database password
    type: password
    generate:
      kind: password
      length: 32
  - key: DB_NAME
    label: Initial database name
    type: string
    default: app

ports:
  - container: 5432
    expose: false

volumes:
  - name: data
    mount: /var/lib/fixture/data

compose: |
  services:
    fixture:
      image: fixture:1-alpine
      restart: unless-stopped
      environment:
        DB_USER: "{{ inputs.DB_USER }}"
        DB_PASSWORD: "{{ inputs.DB_PASSWORD }}"
        DB_NAME: "{{ inputs.DB_NAME }}"
      volumes:
        - {{ volumes.data }}:/var/lib/fixture/data
      networks: [{{ network }}]
  networks:
    {{ network }}:
      external: true

post_install:
  message: |
    host: ployer-{{ app.name }}-fixture
    user: {{ inputs.DB_USER }}
  outputs:
    - label: Internal connection URI
      value: "proto://{{ inputs.DB_USER }}:{{ inputs.DB_PASSWORD }}@ployer-{{ app.name }}-fixture:5432/{{ inputs.DB_NAME }}"
"#;

fn fixture() -> Template {
    serde_yaml::from_str(FIXTURE).expect("fixture template must parse")
}

fn ctx<'a>(app_name: &'a str, inputs: &[(&str, &str)]) -> RenderContext<'a> {
    RenderContext {
        app_name,
        app_id: "app-0001",
        network: "ployer",
        inputs: inputs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

fn render_fixture(app_name: &str, inputs: &[(&str, &str)]) -> Rendered {
    render(&fixture(), ctx(app_name, inputs)).expect("fixture must render")
}

// ---------------------------------------------------------------------------
// Volume naming
//
// Templates reference `{{ volumes.NAME }}` and trust the engine to expand it to
// a host-unique Docker volume name. Two apps installing the same template must
// not land on the same volume, or installing a second database silently adopts
// the first one's data directory.
// ---------------------------------------------------------------------------

#[test]
fn volume_placeholder_expands_to_ployer_prefixed_name() {
    let out = render_fixture("shop", &[]);
    assert!(
        out.compose.contains("ployer_shop_data:/var/lib/fixture/data"),
        "expected the volume mount to use the ployer_<app>_<volume> name, got:\n{}",
        out.compose
    );
}

#[test]
fn different_apps_get_different_volume_names() {
    let first = render_fixture("alpha", &[]);
    let second = render_fixture("beta", &[]);

    assert!(first.compose.contains("ployer_alpha_data:"));
    assert!(second.compose.contains("ployer_beta_data:"));
    assert!(
        !second.compose.contains("ployer_alpha_data:"),
        "a second install must not reuse the first app's volume"
    );
}

// ---------------------------------------------------------------------------
// Credential generation
//
// `generate: {kind: password}` is the only thing standing between a fresh
// install and a database with a predictable password.
// ---------------------------------------------------------------------------

#[test]
fn generated_password_honours_requested_length() {
    let out = render_fixture("app", &[]);
    let password = &out.resolved_inputs["DB_PASSWORD"];
    assert_eq!(password.len(), 32, "fixture asks for length: 32");
}

#[test]
fn generated_password_is_not_empty() {
    let out = render_fixture("app", &[]);
    assert!(!out.resolved_inputs["DB_PASSWORD"].is_empty());
}

#[test]
fn two_installs_generate_different_passwords() {
    let first = render_fixture("app", &[]);
    let second = render_fixture("app", &[]);
    assert_ne!(
        first.resolved_inputs["DB_PASSWORD"], second.resolved_inputs["DB_PASSWORD"],
        "every install must get its own password"
    );
}

#[test]
fn generated_passwords_do_not_repeat_across_many_installs() {
    // One pair differing could be luck; 200 distinct values could not.
    // Guards against a future change to a seeded or time-derived RNG.
    let mut seen = std::collections::HashSet::new();
    for _ in 0..200 {
        seen.insert(render_fixture("app", &[]).resolved_inputs["DB_PASSWORD"].clone());
    }
    assert_eq!(seen.len(), 200, "generated passwords must all be distinct");
}

#[test]
fn generated_password_is_uri_safe() {
    // The password is interpolated into a connection URI shown by the dashboard
    // and pasted into clients verbatim. A '/', '@' or ':' would split the URI.
    for _ in 0..50 {
        let password = render_fixture("app", &[]).resolved_inputs["DB_PASSWORD"].clone();
        assert!(
            password.chars().all(|c| c.is_ascii_alphanumeric()),
            "password must be URI-safe without escaping, got {password:?}"
        );
    }
}

#[test]
fn hex_generator_produces_requested_length_of_hex_digits() {
    let g = Generate {
        kind: GenerateKind::Hex,
        length: Some(31), // odd length exercises the div_ceil + truncate path
    };
    let value = generate(&g);
    assert_eq!(value.len(), 31);
    assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generator_defaults_to_32_when_length_is_omitted() {
    let g = Generate {
        kind: GenerateKind::Password,
        length: None,
    };
    assert_eq!(generate(&g).len(), 32);
}

// ---------------------------------------------------------------------------
// Input resolution
// ---------------------------------------------------------------------------

#[test]
fn user_supplied_input_overrides_the_default() {
    let out = render_fixture("app", &[("DB_NAME", "inventory")]);
    assert_eq!(out.resolved_inputs["DB_NAME"], "inventory");
}

#[test]
fn user_supplied_password_is_used_instead_of_generating_one() {
    let out = render_fixture("app", &[("DB_PASSWORD", "chosen-by-user")]);
    assert_eq!(out.resolved_inputs["DB_PASSWORD"], "chosen-by-user");
}

#[test]
fn omitted_input_falls_back_to_its_default() {
    let out = render_fixture("app", &[]);
    assert_eq!(out.resolved_inputs["DB_USER"], "app");
}

#[test]
fn empty_submitted_value_falls_back_instead_of_being_used() {
    // The install form posts "" for untouched fields. Taking that literally
    // would create a database user with an empty password.
    let out = render_fixture("app", &[("DB_USER", ""), ("DB_PASSWORD", "")]);
    assert_eq!(out.resolved_inputs["DB_USER"], "app");
    assert_eq!(out.resolved_inputs["DB_PASSWORD"].len(), 32);
}

#[test]
fn input_with_neither_default_nor_generator_is_rejected() {
    let mut template = fixture();
    template.inputs.push(Input {
        key: "REQUIRED_THING".to_string(),
        label: "Required".to_string(),
        kind: InputKind::String,
        default: None,
        generate: None,
        required: Some(true),
    });

    let err = match render(&template, ctx("app", &[])) {
        Err(e) => e,
        Ok(_) => panic!("a template input with no default and no generator must not render"),
    };
    assert!(
        matches!(&err, TemplateError::MissingInput(k) if k == "REQUIRED_THING"),
        "expected MissingInput(REQUIRED_THING), got {err:?}"
    );
}

#[test]
fn non_numeric_value_for_a_number_input_is_rejected() {
    let mut template = fixture();
    template.inputs.push(Input {
        key: "MAX_CONNECTIONS".to_string(),
        label: "Max connections".to_string(),
        kind: InputKind::Number,
        default: None,
        generate: None,
        required: Some(true),
    });

    let err = match render(&template, ctx("app", &[("MAX_CONNECTIONS", "lots")])) {
        Err(e) => e,
        Ok(_) => panic!("a non-numeric value for a number input must not render"),
    };
    assert!(
        matches!(&err, TemplateError::InvalidInput { key, .. } if key == "MAX_CONNECTIONS"),
        "expected InvalidInput for MAX_CONNECTIONS, got {err:?}"
    );
}

#[test]
fn number_input_accepts_an_integer_string() {
    let mut template = fixture();
    template.inputs.push(Input {
        key: "MAX_CONNECTIONS".to_string(),
        label: "Max connections".to_string(),
        kind: InputKind::Number,
        default: None,
        generate: None,
        required: Some(true),
    });

    let out = render(&template, ctx("app", &[("MAX_CONNECTIONS", "100")]))
        .expect("100 is a number");
    assert_eq!(out.resolved_inputs["MAX_CONNECTIONS"], "100");
}

#[test]
fn inputs_not_declared_by_the_template_are_discarded() {
    // The install endpoint forwards whatever the client posted. Unknown keys
    // must not reach the container as environment variables.
    let out = render_fixture("app", &[("SMUGGLED", "value")]);
    assert!(!out.resolved_inputs.contains_key("SMUGGLED"));
    assert!(!out.compose.contains("SMUGGLED"));
}

// ---------------------------------------------------------------------------
// Placeholder substitution
// ---------------------------------------------------------------------------

#[test]
fn network_placeholder_is_substituted_everywhere_it_appears() {
    let out = render_fixture("app", &[]);
    assert!(out.compose.contains("networks: [ployer]"));
    assert!(
        !out.compose.contains("{{"),
        "no placeholder may survive rendering:\n{}",
        out.compose
    );
}

#[test]
fn app_name_and_id_placeholders_are_substituted() {
    let template = fixture();
    let rendered = render(&template, ctx("shop", &[])).unwrap();
    assert!(rendered
        .post_install_message
        .as_ref()
        .unwrap()
        .contains("host: ployer-shop-fixture"));

    let out = substitute(
        "{{ app.id }}",
        &Default::default(),
        &Default::default(),
        "shop",
        "shop",
        "app-0001",
        "ployer",
    );
    assert_eq!(out, "app-0001");
}

#[test]
fn unknown_placeholder_is_left_untouched() {
    // Better a visible `{{ inputs.TYPO }}` in the compose than a silent empty
    // value that starts a container with a blank password.
    let out = substitute(
        "value={{ inputs.TYPO }}",
        &Default::default(),
        &Default::default(),
        "app",
        "app",
        "id",
        "net",
    );
    assert_eq!(out, "value={{inputs.TYPO}}");
}

#[test]
fn unclosed_placeholder_does_not_hang_or_panic() {
    let out = substitute(
        "prefix {{ inputs.NEVER_CLOSED",
        &Default::default(),
        &Default::default(),
        "app",
        "app",
        "id",
        "net",
    );
    assert!(out.contains("prefix "));
    assert!(out.contains("NEVER_CLOSED"));
}

#[test]
fn whitespace_inside_the_placeholder_braces_is_tolerated() {
    let mut inputs = std::collections::HashMap::new();
    inputs.insert("K".to_string(), "v".to_string());
    for form in ["{{inputs.K}}", "{{ inputs.K }}", "{{   inputs.K   }}"] {
        let out = substitute(form, &inputs, &Default::default(), "a", "a", "i", "n");
        assert_eq!(out, "v", "failed for {form}");
    }
}

// ---------------------------------------------------------------------------
// post_install outputs — what the dashboard shows the user after installing
// ---------------------------------------------------------------------------

#[test]
fn connection_uri_output_carries_the_generated_password() {
    // If this breaks, the dashboard shows a URI the user cannot connect with.
    let out = render_fixture("shop", &[("DB_USER", "svc"), ("DB_NAME", "orders")]);
    let password = &out.resolved_inputs["DB_PASSWORD"];

    let (label, uri) = out
        .outputs
        .iter()
        .find(|(l, _)| l == "Internal connection URI")
        .expect("template declares a connection URI output");

    assert_eq!(label, "Internal connection URI");
    assert_eq!(
        uri,
        &format!("proto://svc:{password}@ployer-shop-fixture:5432/orders")
    );
}

#[test]
fn template_without_post_install_renders_no_outputs() {
    let mut template = fixture();
    template.post_install = None;

    let out = render(&template, ctx("app", &[])).unwrap();
    assert!(out.post_install_message.is_none());
    assert!(out.outputs.is_empty());
}

#[cfg(test)]
mod password_in_compose {
    use super::*;

    #[test]
    fn resolved_password_is_the_one_written_into_the_compose() {
        // resolved_inputs is persisted and shown to the user; the compose is what
        // actually starts the container. They must agree.
        let out = render_fixture("app", &[]);
        let password = &out.resolved_inputs["DB_PASSWORD"];
        assert!(
            out.compose.contains(&format!("DB_PASSWORD: \"{password}\"")),
            "compose must carry the same password that was reported to the user"
        );
    }
}

// ---------------------------------------------------------------------------
// {{ app.slug }} placeholder
//
// From `0c80422` on main. Container names are built from the slugified app
// name, so this is the only placeholder whose output Docker DNS will answer
// for. Kept alongside the engine tests above rather than in a second file.
// ---------------------------------------------------------------------------

/// Minimal template whose compose/message/outputs all echo the same expression,
/// so one fixture exercises every `substitute()` call site in `render()`.
fn template_echoing(expr: &str) -> Template {
    Template {
        slug: "postgresql".to_string(),
        name: "PostgreSQL".to_string(),
        description: String::new(),
        category: "database".to_string(),
        tags: Vec::new(),
        version: serde_json::Value::String("1.0.0".to_string()),
        inputs: Vec::new(),
        ports: Vec::new(),
        volumes: vec![Volume {
            name: "data".to_string(),
            mount: "/var/lib/postgresql/data".to_string(),
        }],
        compose: format!("compose:{expr}"),
        post_install: Some(PostInstall {
            message: Some(format!("message:{expr}")),
            outputs: vec![OutputItem {
                label: "Host".to_string(),
                value: expr.to_string(),
            }],
        }),
    }
}


/// An `Application` as `deployment.rs` sees it, so the container-name assertion
/// goes through the real `Application::slug()` rather than a copy of it.
fn application_named(name: &str) -> Application {
    Application {
        id: "app-id-0001".to_string(),
        name: name.to_string(),
        server_id: "server-1".to_string(),
        git_url: None,
        git_branch: "main".to_string(),
        build_strategy: BuildStrategy::DockerCompose,
        dockerfile_path: None,
        port: None,
        cpu_limit: None,
        memory_limit: None,
        status: AppStatus::Idle,
        auto_deploy: false,
        compose_content: None,
        template_slug: Some("postgresql".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn app_slug_resolves_everywhere_a_placeholder_is_substituted() {
    let template = template_echoing("{{ app.slug }}");
    let rendered = render(&template, ctx("My Service App", &[])).expect("render");

    assert_eq!(rendered.compose, "compose:my-service-app");
    assert_eq!(
        rendered.post_install_message.as_deref(),
        Some("message:my-service-app")
    );
    assert_eq!(
        rendered.outputs,
        vec![("Host".to_string(), "my-service-app".to_string())]
    );
}

/// The point of the placeholder: a mixed-case, spaced app name must render the
/// exact container name `deployment.rs` creates
/// (`format!("ployer-{}-{}", application.slug(), service_name)`), because that
/// is the only name Docker's embedded DNS answers for.
#[test]
fn app_slug_renders_the_container_name_deployment_uses() {
    let app_name = "SLW Homes  DB";
    let service_name = "postgresql";

    let template = template_echoing("ployer-{{ app.slug }}-postgresql");
    let rendered = render(&template, ctx(app_name, &[])).expect("render");

    let container_name = format!(
        "ployer-{}-{}",
        application_named(app_name).slug(),
        service_name
    );

    assert_eq!(container_name, "ployer-slw-homes-db-postgresql");
    assert_eq!(rendered.compose, format!("compose:{container_name}"));
    assert_eq!(rendered.outputs, vec![("Host".to_string(), container_name)]);
}

/// `{{ app.slug }}` is a pure addition: `{{ app.name }}` must keep emitting the
/// raw, unslugified name so existing templates do not change meaning.
#[test]
fn app_name_still_renders_the_raw_name() {
    let template = template_echoing("{{ app.name }}");
    let rendered = render(&template, ctx("My Service App", &[])).expect("render");

    assert_eq!(rendered.compose, "compose:My Service App");
}

/// Documents the rollout ordering constraint: a binary that does not know a
/// placeholder re-emits it literally rather than erroring, so templates using
/// `{{ app.slug }}` must only be published after this change ships.
#[test]
fn unknown_placeholder_is_re_emitted_literally() {
    let template = template_echoing("{{ app.unknown }}");
    let rendered = render(&template, ctx("My Service App", &[])).expect("render");

    assert_eq!(rendered.compose, "compose:{{app.unknown}}");
}
