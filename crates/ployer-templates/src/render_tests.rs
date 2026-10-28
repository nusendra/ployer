use std::collections::HashMap;

use chrono::Utc;
use ployer_core::models::{AppStatus, Application, BuildStrategy};

use super::*;
use crate::schema::{OutputItem, PostInstall, Volume};

/// Minimal template whose compose/message/outputs all echo the same expression,
/// so one fixture exercises every `substitute()` call site in `render()`.
fn template_echoing(expr: &str) -> Template {
    Template {
        slug: "postgresql".to_string(),
        name: "PostgreSQL".to_string(),
        description: String::new(),
        category: "database".to_string(),
        icon: None,
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

fn ctx(app_name: &str) -> RenderContext<'_> {
    RenderContext {
        app_name,
        app_id: "app-id-0001",
        network: "ployer",
        inputs: HashMap::new(),
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
    let rendered = render(&template, ctx("My Service App")).expect("render");

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
    let rendered = render(&template, ctx(app_name)).expect("render");

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
    let rendered = render(&template, ctx("My Service App")).expect("render");

    assert_eq!(rendered.compose, "compose:My Service App");
}

/// Documents the rollout ordering constraint: a binary that does not know a
/// placeholder re-emits it literally rather than erroring, so templates using
/// `{{ app.slug }}` must only be published after this change ships.
#[test]
fn unknown_placeholder_is_re_emitted_literally() {
    let template = template_echoing("{{ app.unknown }}");
    let rendered = render(&template, ctx("My Service App")).expect("render");

    assert_eq!(rendered.compose, "compose:{{app.unknown}}");
}
