use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use ployer_core::models::{WebhookProvider, WebhookDeliveryStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::app_state::SharedState;
use crate::auth::AuthUser;
use crate::services::webhook::{
    parse_github_push, parse_gitlab_push, verify_github_signature, verify_gitlab_signature,
};
use crate::services::DeploymentService;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/applications/:app_id/webhooks",
            get(get_webhook).post(create_webhook).delete(delete_webhook),
        )
        .route(
            "/applications/:app_id/webhooks/deliveries",
            get(list_deliveries),
        )
        .route("/webhooks/github", post(handle_github_webhook))
        .route("/webhooks/gitlab", post(handle_gitlab_webhook))
}

#[derive(Debug, Deserialize)]
struct CreateWebhookRequest {
    provider: WebhookProvider,
}

#[derive(Debug, Deserialize)]
struct WebhookQuery {
    app_id: String,
}

#[derive(Debug, Serialize)]
struct WebhookResponse {
    id: String,
    application_id: String,
    provider: WebhookProvider,
    webhook_url: String,
    secret: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct DeliveryResponse {
    id: String,
    provider: WebhookProvider,
    event_type: String,
    branch: Option<String>,
    commit_sha: Option<String>,
    commit_message: Option<String>,
    author: Option<String>,
    status: WebhookDeliveryStatus,
    deployment_id: Option<String>,
    delivered_at: String,
}

/// Create or update webhook for an application
async fn create_webhook(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());
    let app_repo = ployer_db::repositories::ApplicationRepository::new(state.db.clone());

    // Verify application exists
    app_repo
        .get(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Generate webhook secret
    let secret = Uuid::new_v4().to_string();

    // Check if webhook already exists
    let existing = webhook_repo
        .find_by_application(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let webhook = if let Some(_existing) = existing {
        // Update existing webhook
        webhook_repo
            .update_secret(&app_id, &secret)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        webhook_repo
            .find_by_application(&app_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch webhook".to_string()))?
    } else {
        // Create new webhook
        webhook_repo
            .create(&app_id, req.provider.clone(), &secret)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    // Construct webhook URL based on provider
    let webhook_url = match req.provider {
        WebhookProvider::GitHub => format!("{}/api/v1/webhooks/github?app_id={}",
            state.config.server.public_url, app_id),
        WebhookProvider::GitLab => format!("{}/api/v1/webhooks/gitlab?app_id={}",
            state.config.server.public_url, app_id),
    };

    Ok(Json(WebhookResponse {
        id: webhook.id,
        application_id: webhook.application_id,
        provider: webhook.provider,
        webhook_url,
        secret: webhook.secret,
        enabled: webhook.enabled,
    }))
}

/// Get webhook details for an application
async fn get_webhook(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());

    let webhook = webhook_repo
        .find_by_application(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Webhook not found".to_string()))?;

    let webhook_url = match webhook.provider {
        WebhookProvider::GitHub => format!("{}/api/v1/webhooks/github?app_id={}",
            state.config.server.public_url, app_id),
        WebhookProvider::GitLab => format!("{}/api/v1/webhooks/gitlab?app_id={}",
            state.config.server.public_url, app_id),
    };

    Ok(Json(WebhookResponse {
        id: webhook.id,
        application_id: webhook.application_id,
        provider: webhook.provider,
        webhook_url,
        secret: webhook.secret,
        enabled: webhook.enabled,
    }))
}

/// Delete webhook for an application
async fn delete_webhook(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());

    webhook_repo
        .delete(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// List webhook deliveries for an application
async fn list_deliveries(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());

    let deliveries = webhook_repo
        .list_deliveries(&app_id, 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<DeliveryResponse> = deliveries
        .into_iter()
        .map(|d| DeliveryResponse {
            id: d.id,
            provider: d.provider,
            event_type: d.event_type,
            branch: d.branch,
            commit_sha: d.commit_sha,
            commit_message: d.commit_message,
            author: d.author,
            status: d.status,
            deployment_id: d.deployment_id,
            delivered_at: d.delivered_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(response))
}

/// Handle GitHub webhook
async fn handle_github_webhook(
    State(state): State<SharedState>,
    Query(query): Query<WebhookQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let app_id = &query.app_id;

    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());
    let app_repo = ployer_db::repositories::ApplicationRepository::new(state.db.clone());

    // Get webhook configuration
    let webhook = webhook_repo
        .find_by_application(app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Webhook not configured".to_string()))?;

    if !webhook.enabled {
        return Err((StatusCode::FORBIDDEN, "Webhook is disabled".to_string()));
    }

    // Verify signature
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing signature header".to_string()))?;

    verify_github_signature(&webhook.secret, &body, signature)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    // Parse payload
    let payload = parse_github_push(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Get application to check auto-deploy branch
    let application = app_repo
        .get(app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Check if this is the branch we should auto-deploy
    let should_deploy = application.branch == payload.branch;

    let (status, deployment_id) = if should_deploy {
        // Ensure Docker client is available
        let docker = match &state.docker {
            Some(docker) => docker.clone(),
            None => {
                tracing::error!("Docker client not available for auto-deploy");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, "Docker not available".to_string()));
            }
        };

        // Get deploy key if exists
        let deploy_key_repo = ployer_db::repositories::DeployKeyRepository::new(state.db.clone());
        let private_key = match deploy_key_repo.get(&application.id).await {
            Ok(Some(key)) => Some(key.private_key),
            _ => None,
        };

        // Trigger deployment
        let deploy_service = DeploymentService::new(
            state.db.clone(),
            docker,
            Some(Arc::new(state.caddy.clone())),
            state.config.server.base_domain.clone(),
            state.ws_broadcast.clone(),
        );

        match deploy_service.deploy(application.clone(), private_key).await {
            Ok(deployment) => {
                tracing::info!("Auto-deploy triggered for app {} via GitHub webhook", app_id);
                (WebhookDeliveryStatus::Success, Some(deployment.id))
            }
            Err(e) => {
                tracing::error!("Auto-deploy failed for app {}: {}", app_id, e);
                (WebhookDeliveryStatus::Failed, None)
            }
        }
    } else {
        (WebhookDeliveryStatus::Skipped, None)
    };

    // Record delivery
    webhook_repo
        .create_delivery(
            &webhook.id,
            app_id,
            WebhookProvider::GitHub,
            "push",
            Some(&payload.branch),
            Some(&payload.commit_sha),
            Some(&payload.commit_message),
            Some(&payload.author),
            status,
            Some(200),
            None,
            deployment_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Handle GitLab webhook
async fn handle_gitlab_webhook(
    State(state): State<SharedState>,
    Query(query): Query<WebhookQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let app_id = &query.app_id;

    let webhook_repo = ployer_db::repositories::WebhookRepository::new(state.db.clone());
    let app_repo = ployer_db::repositories::ApplicationRepository::new(state.db.clone());

    // Get webhook configuration
    let webhook = webhook_repo
        .find_by_application(app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Webhook not configured".to_string()))?;

    if !webhook.enabled {
        return Err((StatusCode::FORBIDDEN, "Webhook is disabled".to_string()));
    }

    // Verify token
    let token = headers
        .get("x-gitlab-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing GitLab token header".to_string()))?;

    verify_gitlab_signature(&webhook.secret, token)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    // Parse payload
    let payload = parse_gitlab_push(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Get application to check auto-deploy branch
    let application = app_repo
        .get(app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Check if this is the branch we should auto-deploy
    let should_deploy = application.branch == payload.branch;

    let (status, deployment_id) = if should_deploy {
        // Ensure Docker client is available
        let docker = match &state.docker {
            Some(docker) => docker.clone(),
            None => {
                tracing::error!("Docker client not available for auto-deploy");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, "Docker not available".to_string()));
            }
        };

        // Get deploy key if exists
        let deploy_key_repo = ployer_db::repositories::DeployKeyRepository::new(state.db.clone());
        let private_key = match deploy_key_repo.get(&application.id).await {
            Ok(Some(key)) => Some(key.private_key),
            _ => None,
        };

        // Trigger deployment
        let deploy_service = DeploymentService::new(
            state.db.clone(),
            docker,
            Some(Arc::new(state.caddy.clone())),
            state.config.server.base_domain.clone(),
            state.ws_broadcast.clone(),
        );

        match deploy_service.deploy(application.clone(), private_key).await {
            Ok(deployment) => {
                tracing::info!("Auto-deploy triggered for app {} via GitLab webhook", app_id);
                (WebhookDeliveryStatus::Success, Some(deployment.id))
            }
            Err(e) => {
                tracing::error!("Auto-deploy failed for app {}: {}", app_id, e);
                (WebhookDeliveryStatus::Failed, None)
            }
        }
    } else {
        (WebhookDeliveryStatus::Skipped, None)
    };

    // Record delivery
    webhook_repo
        .create_delivery(
            &webhook.id,
            app_id,
            WebhookProvider::GitLab,
            "push",
            Some(&payload.branch),
            Some(&payload.commit_sha),
            Some(&payload.commit_message),
            Some(&payload.author),
            status,
            Some(200),
            None,
            deployment_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
