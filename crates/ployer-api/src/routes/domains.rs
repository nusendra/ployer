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
        .route("/applications/:app_id/domains/:domain/wildcard", post(set_wildcard_domain))
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct AddDomainRequest {
    domain: String,
    #[serde(default)]
    is_primary: bool,
    /// Serve as a wildcard (`*.<domain>`) plus the apex, for tenant subdomains.
    /// The route (with HTTPS if a Cloudflare token is configured) is applied on
    /// the next deploy.
    #[serde(default)]
    wildcard: bool,
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

/// Normalize user-entered domains into a bare hostname. Strips a scheme
/// (`https://`), any path/port, a leading `*.` wildcard label, surrounding
/// whitespace and a trailing dot, and lowercases. A hostname with slashes would
/// otherwise inject extra path segments and break the domain's REST routes.
fn normalize_domain(input: &str) -> String {
    let s = input.trim();
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    let s = s.split('/').next().unwrap_or(s); // drop any path
    let s = s.split(':').next().unwrap_or(s); // drop any port
    let s = s.strip_prefix("*.").unwrap_or(s); // wildcard is a separate flag
    s.trim().trim_end_matches('.').to_lowercase()
}

/// A valid hostname: labels of [a-z0-9-] separated by dots, at least one dot.
fn is_valid_hostname(host: &str) -> bool {
    !host.is_empty()
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

async fn add_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<AddDomainRequest>,
) -> Result<(StatusCode, Json<DomainResponse>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Normalize and validate the hostname.
    let domain_name = normalize_domain(&req.domain);
    if domain_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Domain name is required".to_string()));
    }
    if !is_valid_hostname(&domain_name) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("'{}' is not a valid domain name. Enter a bare hostname like example.com.", domain_name),
        ));
    }

    let repo = DomainRepository::new(state.db.clone());

    // Check if domain already exists
    if let Some(_) = repo.find_by_domain(&domain_name).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        return Err((StatusCode::CONFLICT, "Domain already exists".to_string()));
    }

    // Create domain
    let domain = repo
        .create(&app_id, &domain_name, req.is_primary, req.wildcard)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Caddy route is written by the deployment service on the next deploy,
    // where the running container's host port is known. Redeploy to apply.

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

#[derive(Debug, Deserialize)]
struct SetWildcardRequest {
    wildcard: bool,
}

async fn set_wildcard_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, domain)): Path<(String, String)>,
    Json(req): Json<SetWildcardRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DomainRepository::new(state.db.clone());

    let domain_record = repo
        .find_by_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Domain not found".to_string()))?;

    if domain_record.application_id != app_id {
        return Err((StatusCode::FORBIDDEN, "Domain does not belong to this application".to_string()));
    }

    repo.set_wildcard(&domain_record.id, req.wildcard)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // The route is (re)written on the next deploy; redeploy to apply.
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{is_valid_hostname, normalize_domain};

    #[test]
    fn normalize_strips_scheme_path_port_and_wildcard() {
        assert_eq!(normalize_domain("https://slw.homes"), "slw.homes");
        assert_eq!(normalize_domain("http://SLW.Homes/login"), "slw.homes");
        assert_eq!(normalize_domain("slw.homes:8080"), "slw.homes");
        assert_eq!(normalize_domain("*.slw.homes"), "slw.homes");
        assert_eq!(normalize_domain("  slw.homes.  "), "slw.homes");
    }

    #[test]
    fn hostname_validation() {
        assert!(is_valid_hostname("slw.homes"));
        assert!(is_valid_hostname("a.b.example.com"));
        assert!(!is_valid_hostname("nodot"));
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("slw.homes/x"));
        assert!(!is_valid_hostname("-bad.com"));
        assert!(!is_valid_hostname("bad-.com"));
    }
}
