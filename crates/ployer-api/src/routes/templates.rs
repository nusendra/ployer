use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use ployer_core::crypto;
use ployer_db::repositories::{ApplicationRepository, EnvVarRepository};
use ployer_templates::{render, Template, TemplateError};
use ployer_templates::render::RenderContext;
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use crate::services::deployment::DeploymentService;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/:slug", get(get_template))
        .route("/:slug/install", post(install_template))
}

#[derive(Debug, Serialize)]
struct ListResponse {
    templates: Vec<CatalogEntry>,
}

#[derive(Debug, Serialize)]
struct CatalogEntry {
    slug: String,
    name: String,
    description: String,
    category: String,
    icon: Option<String>,
    tags: Vec<String>,
}

async fn list_templates(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ListResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let index = state.templates.index().await.map_err(map_err)?;
    let templates = index
        .templates
        .into_iter()
        .map(|e| CatalogEntry {
            slug: e.slug,
            name: e.name,
            description: e.description,
            category: e.category,
            icon: e.icon,
            tags: e.tags,
        })
        .collect();

    Ok(Json(ListResponse { templates }))
}

async fn get_template(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<Template>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let template = state.templates.get(&slug).await.map_err(map_err)?;
    Ok(Json(template))
}

#[derive(Debug, Deserialize)]
struct InstallRequest {
    app_name: String,
    server_id: String,
    #[serde(default)]
    inputs: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct InstallResponse {
    application_id: String,
    deployment_id: String,
    compose: String,
    resolved_inputs: HashMap<String, String>,
    post_install_message: Option<String>,
    outputs: Vec<InstallOutput>,
}

#[derive(Debug, Serialize)]
struct InstallOutput {
    label: String,
    value: String,
}

async fn install_template(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(req): Json<InstallRequest>,
) -> Result<Json<InstallResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let app_name = req.app_name.trim();
    if app_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "app_name required".to_string()));
    }
    if req.server_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "server_id required".to_string()));
    }

    let template = state.templates.get(&slug).await.map_err(map_err)?;

    // Reserve an id so the compose substitution can reference {{ app.id }} if needed.
    let placeholder_id = uuid::Uuid::new_v4().to_string();
    let ctx = RenderContext {
        app_name,
        app_id: &placeholder_id,
        network: "ployer",
        inputs: req.inputs,
    };
    let rendered = render(&template, ctx).map_err(map_err)?;

    // Persist application + env vars.
    let app_repo = ApplicationRepository::new(state.db.clone());
    let application = app_repo
        .create_from_template(app_name, &req.server_id, &slug, &rendered.compose)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let env_repo = EnvVarRepository::new(state.db.clone());
    let secret_key = state.config.get_secret_key();
    for (key, value) in &rendered.resolved_inputs {
        let encrypted = crypto::encrypt(value, &secret_key).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("env encrypt failed: {e}"),
            )
        })?;
        env_repo
            .create(&application.id, key, &encrypted)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Trigger compose deployment.
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "docker not available".to_string(),
            )
        })?
        .clone();

    let deployment_service = DeploymentService::new(
        state.db.clone(),
        docker,
        Some(Arc::new(state.caddy.clone())),
        state.config.server.base_domain.clone(),
        state.config.get_secret_key(),
        state.ws_broadcast.clone(),
    );

    let deployment = deployment_service
        .deploy_compose(application.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InstallResponse {
        application_id: application.id,
        deployment_id: deployment.id,
        compose: rendered.compose,
        resolved_inputs: rendered.resolved_inputs,
        post_install_message: rendered.post_install_message,
        outputs: rendered
            .outputs
            .into_iter()
            .map(|(label, value)| InstallOutput { label, value })
            .collect(),
    }))
}

fn map_err(err: TemplateError) -> (StatusCode, String) {
    match err {
        TemplateError::NotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
        TemplateError::MissingInput(_) | TemplateError::InvalidInput { .. } => {
            (StatusCode::BAD_REQUEST, err.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}
