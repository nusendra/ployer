//! Tests for `POST /api/v1/templates/:slug/install`.
//!
//! The handler is nine steps long and only the first eight are reachable
//! without a live Docker daemon: `AppState.docker` is a concrete
//! `Option<Arc<DockerClient>>`, so a test can only ever supply `None`, which
//! makes step 8 return 503 before `DeploymentService::deploy_compose` runs.
//! Everything these tests assert therefore stops at that gate. See the module
//! handoff for the step-by-step coverage map.
//!
//! The registry is fed from a pre-seeded disk cache. `registry_url` points at a
//! scheme reqwest refuses to dial, so `Registry::index()` fails in the request
//! builder (no socket is opened, no DNS is resolved) and falls back to the
//! on-disk index, which is how these tests stay hermetic and fast.

use super::*;

use axum::http::HeaderValue;
use ployer_core::config::{AppConfig, AuthConfig, TemplatesConfig};
use ployer_core::models::{AppStatus, Application, BuildStrategy, EnvironmentVariable};
use ployer_db::repositories::ServerRepository;
use ployer_proxy::CaddyClient;
use ployer_templates::compose;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::{Path as FsPath, PathBuf};
use std::str::FromStr;

use crate::app_state::AppState;
use crate::auth::jwt::generate_token;

const JWT_SECRET: &str = "install-endpoint-test-secret";

/// A scheme `reqwest` rejects in the request builder. Keeps the registry
/// unreachable without opening a socket or waiting for a timeout.
const UNREACHABLE_REGISTRY: &str = "offline-fixture://registry.invalid";

// ── Fixture templates ─────────────────────────────────────────────────────

/// Two inputs (one generated, one defaulted), one port, one volume.
const SVC_YAML: &str = r#"
slug: svc
name: Demo Service
description: Fixture service with inputs, a port and a volume
category: tools
tags:
  - demo
version: "1.0.0"
inputs:
  - key: ADMIN_PASSWORD
    label: Admin password
    type: password
    generate:
      kind: password
      length: 24
  - key: SITE_URL
    label: Site URL
    type: string
    default: "http://example.test"
ports:
  - container: 8080
volumes:
  - name: data
    mount: /var/lib/demo
compose: |
  services:
    app:
      image: demo/app:1
      environment:
        ADMIN_PASSWORD: "{{ inputs.ADMIN_PASSWORD }}"
        SITE_URL: "{{ inputs.SITE_URL }}"
      volumes:
        - "{{ volumes.data }}:/var/lib/demo"
post_install:
  message: "Open {{ inputs.SITE_URL }}"
  outputs:
    - label: Admin password
      value: "{{ inputs.ADMIN_PASSWORD }}"
"#;

/// Two services and two declared ports, for the "every service" binding rule.
const MULTI_YAML: &str = r#"
slug: multi
name: Multi Service
description: Fixture with two services and two declared ports
category: tools
version: "1.0.0"
ports:
  - container: 8080
  - container: 9090
compose: |
  services:
    web:
      image: demo/web:1
    worker:
      image: demo/worker:1
"#;

/// No inputs, no ports.
const PLAIN_YAML: &str = r#"
slug: plain
name: Plain Service
description: Fixture with no inputs and no declared ports
category: tools
version: "1.0.0"
compose: |
  services:
    app:
      image: demo/plain:1
"#;

/// One input with neither a default nor a generator: unresolvable.
const NEEDS_LICENSE_YAML: &str = r#"
slug: needs-license
name: Licensed Service
description: Fixture whose only input cannot be resolved without the caller
category: tools
version: "1.0.0"
inputs:
  - key: LICENSE_KEY
    label: License key
    type: string
    required: true
compose: |
  services:
    app:
      image: demo/licensed:1
      environment:
        LICENSE_KEY: "{{ inputs.LICENSE_KEY }}"
"#;

/// One numeric input, so a non-numeric value is rejected.
const TUNED_YAML: &str = r#"
slug: tuned
name: Tuned Service
description: Fixture with a numeric input
category: tools
version: "1.0.0"
inputs:
  - key: MAX_CONNECTIONS
    label: Max connections
    type: number
    default: "10"
compose: |
  services:
    app:
      image: demo/tuned:1
      environment:
        MAX_CONNECTIONS: "{{ inputs.MAX_CONNECTIONS }}"
"#;

const FIXTURES: &[(&str, &str)] = &[
    ("svc", SVC_YAML),
    ("multi", MULTI_YAML),
    ("plain", PLAIN_YAML),
    ("needs-license", NEEDS_LICENSE_YAML),
    ("tuned", TUNED_YAML),
];

// ── Harness ───────────────────────────────────────────────────────────────

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Write the fixture catalog where `Registry` looks for its disk cache. The
/// file names and the sha256 in the index must match what `Registry` recomputes
/// or it treats the cache as corrupt and goes to the network.
fn seed_registry_cache(dir: &FsPath) {
    std::fs::create_dir_all(dir).expect("create template cache dir");

    let entries: Vec<serde_json::Value> = FIXTURES
        .iter()
        .map(|(slug, yaml)| {
            let sha = sha256_hex(yaml.as_bytes());
            std::fs::write(dir.join(format!("{}-{}.yaml", slug, &sha[..16])), yaml)
                .expect("write template fixture");
            serde_json::json!({
                "slug": slug,
                "name": slug,
                "description": "fixture",
                "category": "tools",
                "tags": [],
                "version": "1.0.0",
                "sha256": sha,
                "url": format!("{UNREACHABLE_REGISTRY}/{slug}.yaml"),
            })
        })
        .collect();

    let index = serde_json::json!({
        "schema_version": 1,
        "generated_at": "2026-01-01T00:00:00Z",
        "templates": entries,
    });
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&index).expect("serialize index"),
    )
    .expect("write index fixture");
}

fn test_config(cache_dir: &FsPath) -> AppConfig {
    AppConfig {
        auth: AuthConfig {
            jwt_secret: JWT_SECRET.to_string(),
            token_expiry_hours: 1,
        },
        templates: TemplatesConfig {
            registry_url: UNREACHABLE_REGISTRY.to_string(),
            cache_dir: cache_dir.to_string_lossy().into_owned(),
            index_ttl_seconds: 60,
        },
        ..AppConfig::default()
    }
}

/// In-memory pool with the production pragmas. `max_connections(1)` is load
/// bearing: every extra connection to `sqlite::memory:` is a *different*
/// database. `foreign_keys(true)` mirrors `ployer_db::create_pool`, so the
/// `applications.server_id` constraint is enforced here as it is in production.
async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("parse sqlite url")
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("open in-memory sqlite pool");
    ployer_db::run_migrations(&pool).await.expect("migrations");
    pool
}

struct TestEnv {
    state: SharedState,
    config: AppConfig,
    server_id: String,
    cache_dir: PathBuf,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.cache_dir);
    }
}

impl TestEnv {
    async fn new() -> Self {
        let cache_dir =
            std::env::temp_dir().join(format!("ployer-install-tests-{}", uuid::Uuid::new_v4()));
        seed_registry_cache(&cache_dir);

        let config = test_config(&cache_dir);
        let pool = test_pool().await;

        let server_id = ServerRepository::new(pool.clone())
            .create("test-server", "127.0.0.1", 22, "root", None, true)
            .await
            .expect("create server")
            .id;

        // `docker: None` is not a convenience: `DockerClient` is a concrete
        // bollard client with no trait seam, so this is the only value a test
        // can supply. It is what caps coverage at step 8.
        let state = AppState::new(
            pool,
            None,
            CaddyClient::new("http://127.0.0.1:1", "/dev/null"),
            config.clone(),
        );

        Self {
            state,
            config,
            server_id,
            cache_dir,
        }
    }

    fn auth_headers(&self) -> HeaderMap {
        let token = generate_token("user-1", "user@example.com", "admin", JWT_SECRET, 1)
            .expect("generate token");
        headers_with(&format!("Bearer {token}"))
    }

    async fn applications(&self) -> Vec<Application> {
        ApplicationRepository::new(self.state.db.clone())
            .list()
            .await
            .expect("list applications")
    }

    async fn env_vars(&self, application_id: &str) -> Vec<EnvironmentVariable> {
        EnvVarRepository::new(self.state.db.clone())
            .list_by_application(application_id)
            .await
            .expect("list env vars")
    }

    fn decrypt(&self, value: &str) -> String {
        crypto::decrypt(value, &self.config.get_secret_key()).expect("decrypt env var")
    }
}

fn headers_with(authorization: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(authorization).expect("header value"),
    );
    headers
}

fn request(app_name: &str, server_id: &str) -> InstallRequest {
    InstallRequest {
        app_name: app_name.to_string(),
        server_id: server_id.to_string(),
        inputs: HashMap::new(),
        expose: false,
        host_ports: HashMap::new(),
    }
}

async fn call_install(
    env: &TestEnv,
    slug: &str,
    headers: HeaderMap,
    req: InstallRequest,
) -> Result<Json<InstallResponse>, (StatusCode, String)> {
    install_template(
        State(env.state.clone()),
        headers,
        Path(slug.to_string()),
        Json(req),
    )
    .await
}

/// Authenticated install that is expected to fail somewhere in steps 2-7.
async fn install_err(env: &TestEnv, slug: &str, req: InstallRequest) -> (StatusCode, String) {
    call_install(env, slug, env.auth_headers(), req)
        .await
        .expect_err("install cannot succeed without a docker client")
}

/// Drive the handler through steps 1-7 and stop at the Docker gate (step 8),
/// which always fires because `docker` is `None`. Returns the application row
/// the handler persisted on the way there.
async fn install_up_to_the_docker_gate(
    env: &TestEnv,
    slug: &str,
    req: InstallRequest,
) -> Application {
    let (status, message) = install_err(env, slug, req).await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "expected to reach the docker gate, got: {message}"
    );

    let apps = env.applications().await;
    assert_eq!(apps.len(), 1, "expected exactly one persisted application");
    apps.into_iter().next().expect("application")
}

fn service_ports(compose_yaml: &str, service: &str) -> Vec<String> {
    compose::parse(compose_yaml)
        .expect("parse rendered compose")
        .services
        .remove(service)
        .unwrap_or_else(|| panic!("compose has no service `{service}`"))
        .ports
}

// ── Step 1: authentication ────────────────────────────────────────────────

#[tokio::test]
async fn install_without_an_authorization_header_is_unauthorized() {
    let env = TestEnv::new().await;

    let (status, _) = call_install(&env, "svc", HeaderMap::new(), request("app", &env.server_id))
        .await
        .expect_err("missing credentials must be rejected");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn install_with_a_non_bearer_authorization_header_is_unauthorized() {
    let env = TestEnv::new().await;
    let headers = headers_with("Basic dXNlcjpwYXNz");

    let (status, _) = call_install(&env, "svc", headers, request("app", &env.server_id))
        .await
        .expect_err("non-bearer credentials must be rejected");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn install_with_a_token_signed_by_a_different_secret_is_unauthorized() {
    let env = TestEnv::new().await;
    let foreign = generate_token("user-1", "user@example.com", "admin", "another-secret", 1)
        .expect("generate token");
    let headers = headers_with(&format!("Bearer {foreign}"));

    let (status, _) = call_install(&env, "svc", headers, request("app", &env.server_id))
        .await
        .expect_err("a token signed by another secret must be rejected");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthorized_install_persists_nothing() {
    let env = TestEnv::new().await;

    let _ = call_install(&env, "svc", HeaderMap::new(), request("app", &env.server_id)).await;

    assert!(
        env.applications().await.is_empty(),
        "auth must be checked before anything is written"
    );
}

// ── Step 2: request validation ────────────────────────────────────────────

#[tokio::test]
async fn empty_app_name_is_rejected() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("", &env.server_id)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn whitespace_only_app_name_is_rejected() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("   \t ", &env.server_id)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn empty_server_id_is_rejected() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("app", "")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn whitespace_only_server_id_is_rejected() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("app", "  ")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejected_app_name_persists_nothing() {
    let env = TestEnv::new().await;

    let _ = install_err(&env, "svc", request("", &env.server_id)).await;

    assert!(env.applications().await.is_empty());
}

// ── Step 3: template lookup ───────────────────────────────────────────────

#[tokio::test]
async fn unknown_slug_is_not_found() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "no-such-template", request("app", &env.server_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_slug_persists_nothing() {
    let env = TestEnv::new().await;

    let _ = install_err(&env, "no-such-template", request("app", &env.server_id)).await;

    assert!(env.applications().await.is_empty());
}

// ── Step 4: render ────────────────────────────────────────────────────────

#[tokio::test]
async fn missing_required_input_is_a_bad_request() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "needs-license", request("app", &env.server_id)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn non_numeric_value_for_a_number_input_is_a_bad_request() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.inputs
        .insert("MAX_CONNECTIONS".to_string(), "lots".to_string());

    let (status, _) = install_err(&env, "tuned", req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn failed_render_persists_nothing() {
    let env = TestEnv::new().await;

    let _ = install_err(&env, "needs-license", request("app", &env.server_id)).await;

    assert!(
        env.applications().await.is_empty(),
        "nothing may be written before the template renders"
    );
}

#[tokio::test]
async fn supplied_input_is_substituted_into_the_compose() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.inputs.insert(
        "SITE_URL".to_string(),
        "https://caller.example".to_string(),
    );

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;

    assert!(
        app.compose_content
            .expect("compose stored")
            .contains("https://caller.example"),
        "the caller's value must replace the template default"
    );
}

#[tokio::test]
async fn omitted_input_falls_back_to_the_template_default() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("app", &env.server_id)).await;

    assert!(app
        .compose_content
        .expect("compose stored")
        .contains("http://example.test"));
}

#[tokio::test]
async fn empty_string_input_falls_back_to_the_template_default() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.inputs.insert("SITE_URL".to_string(), String::new());

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;

    assert!(
        app.compose_content
            .expect("compose stored")
            .contains("http://example.test"),
        "an empty value is treated as absent, not as an empty setting"
    );
}

// ── Step 5: port exposure ─────────────────────────────────────────────────

#[tokio::test]
async fn expose_publishes_each_template_port_on_the_matching_host_port() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.expose = true;

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;

    assert_eq!(
        service_ports(&app.compose_content.expect("compose stored"), "app"),
        vec!["8080:8080"]
    );
}

#[tokio::test]
async fn expose_honours_an_explicit_host_port_override() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.expose = true;
    req.host_ports.insert(8080, 18080);

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;

    assert_eq!(
        service_ports(&app.compose_content.expect("compose stored"), "app"),
        vec!["18080:8080"]
    );
}

#[tokio::test]
async fn expose_publishes_the_ports_on_every_service() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.expose = true;

    let app = install_up_to_the_docker_gate(&env, "multi", req).await;
    let compose = app.compose_content.expect("compose stored");

    assert_eq!(
        service_ports(&compose, "web"),
        vec!["8080:8080", "9090:9090"]
    );
    assert_eq!(
        service_ports(&compose, "worker"),
        vec!["8080:8080", "9090:9090"]
    );
}

#[tokio::test]
async fn expose_on_a_template_without_ports_leaves_the_compose_unchanged() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.expose = true;

    let app = install_up_to_the_docker_gate(&env, "plain", req).await;
    let compose = app.compose_content.expect("compose stored");

    assert!(
        !compose.contains("ports"),
        "a template with no declared ports must not gain a ports mapping: {compose}"
    );
}

#[tokio::test]
async fn without_expose_the_compose_publishes_no_ports() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("app", &env.server_id)).await;

    assert!(service_ports(&app.compose_content.expect("compose stored"), "app").is_empty());
}

#[tokio::test]
async fn host_ports_are_ignored_when_expose_is_false() {
    let env = TestEnv::new().await;
    let mut req = request("app", &env.server_id);
    req.host_ports.insert(8080, 18080);

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;

    assert!(
        service_ports(&app.compose_content.expect("compose stored"), "app").is_empty(),
        "host_ports must do nothing unless the caller opted into expose"
    );
}

// ── Step 6: application persistence ───────────────────────────────────────

#[tokio::test]
async fn installed_application_is_persisted_with_the_rendered_compose() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("my-app", &env.server_id)).await;

    assert_eq!(app.name, "my-app");
    assert_eq!(app.server_id, env.server_id);
    let compose = app.compose_content.expect("compose stored");
    assert!(
        !compose.contains("{{"),
        "the stored compose must be rendered, not the raw template: {compose}"
    );
}

#[tokio::test]
async fn installed_application_records_the_template_slug() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("my-app", &env.server_id)).await;

    assert_eq!(app.template_slug.as_deref(), Some("svc"));
}

#[tokio::test]
async fn installed_application_uses_the_docker_compose_build_strategy() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("my-app", &env.server_id)).await;

    assert_eq!(app.build_strategy, BuildStrategy::DockerCompose);
    assert_eq!(app.status, AppStatus::Idle);
}

#[tokio::test]
async fn app_name_is_trimmed_before_it_is_persisted() {
    let env = TestEnv::new().await;

    let app =
        install_up_to_the_docker_gate(&env, "svc", request("  my-app  ", &env.server_id)).await;

    assert_eq!(app.name, "my-app");
}

#[tokio::test]
async fn unknown_server_id_is_an_internal_error() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("app", "no-such-server")).await;

    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "a dangling server_id reaches the database as a foreign-key violation \
         rather than being validated up front"
    );
}

// ── Step 7: encrypted env vars ────────────────────────────────────────────

#[tokio::test]
async fn every_resolved_input_is_persisted_as_an_env_var() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("my-app", &env.server_id)).await;

    let mut keys: Vec<String> = env
        .env_vars(&app.id)
        .await
        .into_iter()
        .map(|v| v.key)
        .collect();
    keys.sort();
    assert_eq!(keys, vec!["ADMIN_PASSWORD", "SITE_URL"]);
}

#[tokio::test]
async fn persisted_env_var_round_trips_through_the_config_secret_key() {
    let env = TestEnv::new().await;
    let mut req = request("my-app", &env.server_id);
    req.inputs.insert(
        "SITE_URL".to_string(),
        "https://caller.example".to_string(),
    );

    let app = install_up_to_the_docker_gate(&env, "svc", req).await;
    let stored = env
        .env_vars(&app.id)
        .await
        .into_iter()
        .find(|v| v.key == "SITE_URL")
        .expect("SITE_URL was stored");

    assert_eq!(env.decrypt(&stored.value_encrypted), "https://caller.example");
}

#[tokio::test]
async fn generated_password_is_not_persisted_in_plaintext() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "svc", request("my-app", &env.server_id)).await;
    let stored = env
        .env_vars(&app.id)
        .await
        .into_iter()
        .find(|v| v.key == "ADMIN_PASSWORD")
        .expect("ADMIN_PASSWORD was stored");

    let plaintext = env.decrypt(&stored.value_encrypted);
    assert_eq!(plaintext.len(), 24, "generator honours the template length");
    assert!(
        !stored.value_encrypted.contains(&plaintext),
        "the generated secret must not survive in the stored ciphertext"
    );
    assert!(
        app.compose_content
            .expect("compose stored")
            .contains(&plaintext),
        "the same generated value must reach the compose the container runs"
    );
}

#[tokio::test]
async fn template_without_inputs_persists_no_env_vars() {
    let env = TestEnv::new().await;

    let app = install_up_to_the_docker_gate(&env, "plain", request("my-app", &env.server_id)).await;

    assert!(env.env_vars(&app.id).await.is_empty());
}

// ── Step 8: the Docker gate ───────────────────────────────────────────────

#[tokio::test]
async fn install_without_a_docker_client_is_service_unavailable() {
    let env = TestEnv::new().await;

    let (status, message) = install_err(&env, "plain", request("my-app", &env.server_id)).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(message.contains("docker"), "unexpected message: {message}");
}

// ── Characterisation of known defects ─────────────────────────────────────
//
// These two pin behaviour that is wrong. They exist so the defect is visible in
// the suite rather than silent, and so whoever fixes it gets a loud, clearly
// named failure telling them which test to update. Do not treat a green run
// here as the behaviour being correct.

/// DEFECT: steps 6 and 7 write the application and its env vars *before* step 8
/// checks for Docker, and nothing rolls them back. A Docker-less server answers
/// 503 having already persisted an application that was never deployed.
#[tokio::test]
async fn characterises_defect_a_503_leaves_an_orphaned_application_behind() {
    let env = TestEnv::new().await;

    let (status, _) = install_err(&env, "svc", request("my-app", &env.server_id)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    let apps = env.applications().await;
    assert_eq!(
        apps.len(),
        1,
        "current behaviour: the failed install leaves its application row behind"
    );
    assert!(
        !env.env_vars(&apps[0].id).await.is_empty(),
        "current behaviour: its encrypted env vars are orphaned too"
    );
}

/// DEFECT: nothing rejects a repeated name. Containers are named
/// `ployer-<app.slug>-<service>`, so two applications with the same name render
/// colliding container names.
#[tokio::test]
async fn characterises_defect_the_same_app_name_can_be_installed_twice() {
    let env = TestEnv::new().await;

    let first = install_err(&env, "svc", request("my-app", &env.server_id)).await;
    let second = install_err(&env, "svc", request("my-app", &env.server_id)).await;

    assert_eq!(first.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(second.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        env.applications().await.len(),
        2,
        "current behaviour: duplicate application names are accepted"
    );
}

// ── Request body contract (the axum `Json` extractor layer) ───────────────

#[test]
fn install_request_defaults_inputs_expose_and_host_ports() {
    let req: InstallRequest =
        serde_json::from_str(r#"{"app_name":"a","server_id":"s"}"#).expect("minimal body parses");

    assert!(req.inputs.is_empty());
    assert!(!req.expose);
    assert!(req.host_ports.is_empty());
}

#[test]
fn install_request_without_app_name_is_rejected_by_the_extractor() {
    let parsed = serde_json::from_str::<InstallRequest>(r#"{"server_id":"s"}"#);

    assert!(
        parsed.is_err(),
        "app_name has no default, so the body never reaches the handler"
    );
}
