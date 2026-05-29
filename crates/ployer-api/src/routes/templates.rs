use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use ployer_templates::{render, Template, TemplateError};
use ployer_templates::render::RenderContext;
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::auth::extract_user_id;

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
    #[serde(default)]
    inputs: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct InstallPreview {
    /// Rendered docker-compose YAML, ready to deploy.
    compose: String,
    /// Inputs after defaults and generated values are applied.
    /// NOTE: secrets are included so the UI can show them once on the post-install screen.
    resolved_inputs: HashMap<String, String>,
    post_install_message: Option<String>,
    outputs: Vec<InstallOutput>,
    /// Compose-based deployment is not yet wired through ployer-docker.
    /// For now this endpoint returns the rendered artifact only.
    note: &'static str,
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
) -> Result<Json<InstallPreview>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    if req.app_name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "app_name required".to_string()));
    }

    let template = state.templates.get(&slug).await.map_err(map_err)?;

    let app_id = uuid::Uuid::new_v4().to_string();
    let ctx = RenderContext {
        app_name: &req.app_name,
        app_id: &app_id,
        network: "ployer",
        inputs: req.inputs,
    };

    let rendered = render(&template, ctx).map_err(map_err)?;

    Ok(Json(InstallPreview {
        compose: rendered.compose,
        resolved_inputs: rendered.resolved_inputs,
        post_install_message: rendered.post_install_message,
        outputs: rendered
            .outputs
            .into_iter()
            .map(|(label, value)| InstallOutput { label, value })
            .collect(),
        note: "preview only: compose-based deploy is not yet wired into ployer-docker",
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
