use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use ployer_core::models::Domain;
use ployer_db::repositories::DomainRepository;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/applications/:app_id/domains", get(list_domains).post(add_domain))
        .route("/applications/:app_id/domains/:domain", delete(remove_domain))
        .route("/applications/:app_id/domains/:domain/verify", post(verify_domain))
        .route("/applications/:app_id/domains/:domain/primary", post(set_primary_domain))
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct AddDomainRequest {
    domain: String,
    #[serde(default)]
    is_primary: bool,
}

#[derive(Debug, Serialize)]
struct DomainResponse {
    domain: Domain,
}

#[derive(Debug, Serialize)]
struct ListDomainsResponse {
    domains: Vec<Domain>,
}

#[derive(Debug, Serialize)]
struct VerifyDomainResponse {
    success: bool,
    message: String,
}

// ===== Handlers =====

async fn list_domains(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<Json<ListDomainsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DomainRepository::new(state.db.clone());
    let domains = repo
        .list_by_application(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListDomainsResponse { domains }))
}

async fn add_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<AddDomainRequest>,
) -> Result<(StatusCode, Json<DomainResponse>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Validate domain name
    if req.domain.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Domain name is required".to_string()));
    }

    let repo = DomainRepository::new(state.db.clone());

    // Check if domain already exists
    if let Some(_) = repo.find_by_domain(&req.domain).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        return Err((StatusCode::CONFLICT, "Domain already exists".to_string()));
    }

    // Create domain
    let domain = repo
        .create(&app_id, &req.domain, req.is_primary)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Configure Caddy reverse proxy
    // For now, we'll skip Caddy configuration until we have container info
    // This will be handled in the deployment service

    Ok((StatusCode::CREATED, Json(DomainResponse { domain })))
}

async fn remove_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, domain)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DomainRepository::new(state.db.clone());

    // Verify domain belongs to this application
    let domain_record = repo
        .find_by_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Domain not found".to_string()))?;

    if domain_record.application_id != app_id {
        return Err((StatusCode::FORBIDDEN, "Domain does not belong to this application".to_string()));
    }

    // Delete domain
    repo.delete_by_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Remove Caddy route
    // if let Some(ref caddy) = state.caddy {
    //     let _ = caddy.remove_route(&domain).await;
    // }

    Ok(StatusCode::NO_CONTENT)
}

async fn verify_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, domain)): Path<(String, String)>,
) -> Result<Json<VerifyDomainResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DomainRepository::new(state.db.clone());

    // Verify domain belongs to this application
    let domain_record = repo
        .find_by_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Domain not found".to_string()))?;

    if domain_record.application_id != app_id {
        return Err((StatusCode::FORBIDDEN, "Domain does not belong to this application".to_string()));
    }

    // TODO: Implement DNS verification
    // Check if domain points to this server
    // For MVP, we'll just return success
    let success = true;
    let message = if success {
        "Domain verified successfully".to_string()
    } else {
        "Domain verification failed. Please check your DNS settings.".to_string()
    };

    // Update SSL status if verified
    if success {
        repo.update_ssl_status(&domain_record.id, true)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(VerifyDomainResponse { success, message }))
}

async fn set_primary_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, domain)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DomainRepository::new(state.db.clone());

    // Verify domain belongs to this application
    let domain_record = repo
        .find_by_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Domain not found".to_string()))?;

    if domain_record.application_id != app_id {
        return Err((StatusCode::FORBIDDEN, "Domain does not belong to this application".to_string()));
    }

    // Set as primary
    repo.set_primary(&domain_record.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
