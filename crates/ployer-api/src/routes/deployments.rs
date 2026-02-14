use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use crate::services::DeploymentService;
use ployer_core::models::{Deployment, DeploymentStatus};
use ployer_core::crypto;
use ployer_db::repositories::{ApplicationRepository, DeployKeyRepository, DeploymentRepository};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_deployments))
        .route("/:id", get(get_deployment))
        .route("/:id/cancel", post(cancel_deployment))
}

/// Add deployment routes to application router
pub fn app_deploy_router() -> Router<SharedState> {
    Router::new().route("/applications/:id/deploy", post(trigger_deployment))
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct ListDeploymentsQuery {
    application_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeploymentResponse {
    deployment: Deployment,
}

#[derive(Debug, Serialize)]
struct ListDeploymentsResponse {
    deployments: Vec<Deployment>,
}

// ===== Handlers =====

async fn trigger_deployment(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<(StatusCode, Json<DeploymentResponse>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Get application
    let app_repo = ApplicationRepository::new(state.db.clone());
    let application = app_repo
        .find_by_id(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Get deploy key (private key) if application has git_url
    let private_key = if application.git_url.is_some() {
        let key_repo = DeployKeyRepository::new(state.db.clone());
        if let Some(key) = key_repo
            .find_by_application(&app_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            // Decrypt private key
            let secret_key = state.config.get_secret_key();
            let decrypted = crypto::decrypt(&key.private_key_encrypted, &secret_key)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Decryption failed: {}", e)))?;
            Some(decrypted)
        } else {
            None
        }
    } else {
        None
    };

    // Create deployment service
    let docker = state.docker.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?
        .clone();

    let deployment_service = DeploymentService::new(
        state.db.clone(),
        docker,
        Some(Arc::new(state.caddy.clone())),
        state.config.server.base_domain.clone(),
        state.ws_broadcast.clone(),
    );

    // Trigger deployment
    let deployment = deployment_service
        .deploy(application, private_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(DeploymentResponse { deployment })))
}

async fn list_deployments(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<ListDeploymentsQuery>,
) -> Result<Json<ListDeploymentsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DeploymentRepository::new(state.db.clone());
    let deployments = repo
        .list(query.application_id.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListDeploymentsResponse { deployments }))
}

async fn get_deployment(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<DeploymentResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DeploymentRepository::new(state.db.clone());
    let deployment = repo
        .find_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Deployment not found".to_string()))?;

    Ok(Json(DeploymentResponse { deployment }))
}

async fn cancel_deployment(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let docker = state.docker.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?
        .clone();

    let deployment_service = DeploymentService::new(
        state.db.clone(),
        docker,
        Some(Arc::new(state.caddy.clone())),
        state.config.server.base_domain.clone(),
        state.ws_broadcast.clone(),
    );

    let cancelled = deployment_service
        .cancel_deployment(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if cancelled {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::BAD_REQUEST, "Deployment cannot be cancelled".to_string()))
    }
}
