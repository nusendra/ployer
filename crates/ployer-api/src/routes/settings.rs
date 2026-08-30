use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use ployer_db::repositories::SettingsRepository;
use ployer_proxy::cloudflare::{self, CloudflareClient, RecordOutcome, Zone};
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use crate::routes::domains::{is_valid_hostname, normalize_domain};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/dashboard-domain", post(set_dashboard_domain).delete(clear_dashboard_domain))
        .route("/cloudflare/zones", get(list_cloudflare_zones))
}

/// Shared-IP wildcard DNS services. A dashboard on one of these is the
/// install-time default, not a domain the user chose.
fn is_shared_ip_dns(domain: &str) -> bool {
    let d = domain.trim_end_matches('.').to_lowercase();
    d.ends_with(".nip.io") || d.ends_with(".sslip.io")
}

#[derive(Debug, Serialize)]
struct SettingsResponse {
    allow_registration: bool,
    /// Whether a Cloudflare API token is configured (the token itself is never
    /// returned).
    cf_api_token_set: bool,
    /// Whether the running Caddy has the Cloudflare DNS plugin. If false, HTTPS
    /// wildcard certs cannot be issued until a plugin build of Caddy is installed.
    cloudflare_plugin_available: bool,
    /// Hostname the dashboard is served on right now.
    dashboard_domain: String,
    /// Every hostname the dashboard answers on — a custom domain keeps the
    /// original `<ip>.nip.io` address as a fallback.
    dashboard_hosts: Vec<String>,
    /// False while the dashboard is still on the install-time nip.io default.
    dashboard_domain_custom: bool,
    /// Public IPv4 of this server — the target of the `A` record a custom
    /// dashboard domain needs. `null` when detection failed and no override is
    /// set.
    server_ip: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    #[serde(default)]
    allow_registration: Option<bool>,
    /// Set/replace the Cloudflare API token. An empty string clears it. Omit to
    /// leave it unchanged.
    #[serde(default)]
    cf_api_token: Option<String>,
    /// Override the detected public IP (NAT, multi-homed hosts, floating IPs).
    #[serde(default)]
    server_ip: Option<String>,
}

/// Resolve the hostname the dashboard is actually served on. The Caddyfile is
/// the source of truth — it is what Caddy loaded — with the boot-time config
/// as a fallback for dev runs that have no Caddyfile.
fn effective_dashboard_hosts(state: &SharedState) -> Vec<String> {
    let hosts = state.caddy.dashboard_hosts();
    if hosts.is_empty() {
        vec![state.config.server.base_domain.clone()]
    } else {
        hosts
    }
}

async fn current_settings(state: &SharedState) -> Result<SettingsResponse, (StatusCode, String)> {
    let repo = SettingsRepository::new(state.db.clone());
    let allow_registration = repo
        .allow_registration()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let server_ip = repo
        .server_public_ip()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let hosts = effective_dashboard_hosts(state);
    let dashboard_domain = hosts.first().cloned().unwrap_or_default();

    Ok(SettingsResponse {
        allow_registration,
        cf_api_token_set: state.caddy.cf_token_is_set(),
        cloudflare_plugin_available: state.caddy.cloudflare_plugin_available(),
        dashboard_domain_custom: !is_shared_ip_dns(&dashboard_domain),
        dashboard_domain,
        dashboard_hosts: hosts,
        server_ip,
    })
}

async fn get_settings(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;
    Ok(Json(current_settings(&state).await?))
}

async fn update_settings(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = SettingsRepository::new(state.db.clone());

    if let Some(allow) = req.allow_registration {
        let value = if allow { "true" } else { "false" };
        repo.set("allow_registration", value)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(token) = req.cf_api_token {
        let trimmed = token.trim().to_string();
        repo.set("cf_api_token", &trimmed)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        // Apply immediately so new wildcard deploys pick it up without a restart.
        let token_opt = if trimmed.is_empty() { None } else { Some(trimmed) };
        state.caddy.set_cf_token(token_opt);
    }

    if let Some(ip) = req.server_ip {
        let trimmed = ip.trim();
        if !trimmed.is_empty() && !cloudflare::is_ipv4(trimmed) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("'{}' is not a valid IPv4 address.", trimmed),
            ));
        }
        repo.set_server_public_ip(trimmed)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(current_settings(&state).await?))
}

// ===== Dashboard domain =====

#[derive(Debug, Deserialize)]
struct SetDashboardDomainRequest {
    domain: String,
    /// Create/update the `A` record in Cloudflare instead of leaving it to the
    /// user. Requires a configured Cloudflare token whose zone covers the
    /// domain. Defaults to on — it's the point of the feature.
    #[serde(default = "default_true")]
    create_dns_record: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct DnsResult {
    /// `created` | `updated` | `unchanged` | `skipped` | `failed`
    status: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct DashboardDomainResponse {
    domain: String,
    url: String,
    hosts: Vec<String>,
    server_ip: Option<String>,
    dns: DnsResult,
}

/// The server's public IP: the user's override or a stored detection, falling
/// back to a live probe whose result is cached for next time.
async fn resolve_server_ip(repo: &SettingsRepository) -> Option<String> {
    if let Ok(Some(ip)) = repo.server_public_ip().await {
        return Some(ip);
    }
    let detected = cloudflare::detect_public_ip().await?;
    let _ = repo.set_server_public_ip(&detected).await;
    Some(detected)
}

/// Create or fix the `A` record for `domain` in Cloudflare, reporting what
/// happened rather than failing the whole request: DNS is the one step the user
/// can still do by hand, and the Caddy side is worth applying either way.
async fn apply_dns_record(state: &SharedState, domain: &str, server_ip: Option<&str>) -> DnsResult {
    let Some(token) = state.caddy.cf_token() else {
        return DnsResult {
            status: "skipped".to_string(),
            message: "No Cloudflare token configured — add the A record yourself.".to_string(),
        };
    };
    let Some(ip) = server_ip else {
        return DnsResult {
            status: "failed".to_string(),
            message: "Could not determine this server's public IP. Set it in Settings and retry."
                .to_string(),
        };
    };

    let cf = CloudflareClient::new(token);
    let zone = match cf.find_zone(domain).await {
        Ok(Some(zone)) => zone,
        Ok(None) => {
            return DnsResult {
                status: "failed".to_string(),
                message: format!(
                    "No Cloudflare zone for '{}' is visible to this token. Add the A record manually, or use a token scoped to that zone.",
                    domain
                ),
            }
        }
        Err(e) => {
            return DnsResult {
                status: "failed".to_string(),
                message: format!("Cloudflare zone lookup failed: {}", e),
            }
        }
    };

    match cf.upsert_a_record(&zone.id, domain, ip).await {
        Ok(RecordOutcome::Created) => DnsResult {
            status: "created".to_string(),
            message: format!("Created A record {} → {} in zone {}.", domain, ip, zone.name),
        },
        Ok(RecordOutcome::Updated) => DnsResult {
            status: "updated".to_string(),
            message: format!("Updated A record {} → {} (DNS only).", domain, ip),
        },
        Ok(RecordOutcome::Unchanged) => DnsResult {
            status: "unchanged".to_string(),
            message: format!("A record {} already points at {}.", domain, ip),
        },
        Err(e) => DnsResult {
            status: "failed".to_string(),
            message: format!("Cloudflare record write failed: {}", e),
        },
    }
}

async fn set_dashboard_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<SetDashboardDomainRequest>,
) -> Result<Json<DashboardDomainResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let domain = normalize_domain(&req.domain);
    if domain.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Domain name is required".to_string()));
    }
    if !is_valid_hostname(&domain) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("'{}' is not a valid domain name. Enter a bare hostname like ployer.example.com.", domain),
        ));
    }

    let repo = SettingsRepository::new(state.db.clone());
    let server_ip = resolve_server_ip(&repo).await;

    let dns = if req.create_dns_record {
        apply_dns_record(&state, &domain, server_ip.as_deref()).await
    } else {
        DnsResult {
            status: "skipped".to_string(),
            message: "DNS left unchanged at your request.".to_string(),
        }
    };

    // Keep the install-time nip.io address serving the dashboard too, so a
    // domain whose DNS hasn't propagated (or was typo'd) can't lock the user
    // out of the UI they'd use to fix it.
    let fallbacks: Vec<String> = effective_dashboard_hosts(&state)
        .into_iter()
        .filter(|h| is_shared_ip_dns(h))
        .collect();

    state
        .caddy
        .set_dashboard_domain(&domain, &fallbacks)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write Caddyfile: {}", e)))?;

    // Persist so a restart — and the installer's "keep existing domain" path on
    // self-update — hold the new value.
    if let Err(e) = state.caddy.persist_dashboard_domain_env(&domain) {
        tracing::warn!("Failed to persist dashboard domain to ployer.env: {}", e);
    }
    repo.set_dashboard_domain(&domain)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DashboardDomainResponse {
        url: format!("https://{}", domain),
        hosts: state.caddy.dashboard_hosts(),
        domain,
        server_ip,
        dns,
    }))
}

/// Move the dashboard back to the install-time `<ip>.nip.io` address. Used to
/// undo a custom domain without SSHing in to edit the Caddyfile.
async fn clear_dashboard_domain(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<DashboardDomainResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = SettingsRepository::new(state.db.clone());
    let server_ip = resolve_server_ip(&repo).await;

    // Prefer an existing nip.io host (that's the address the install created);
    // otherwise rebuild it from the server IP.
    let fallback = effective_dashboard_hosts(&state)
        .into_iter()
        .find(|h| is_shared_ip_dns(h))
        .or_else(|| server_ip.as_ref().map(|ip| format!("{}.nip.io", ip)))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "No nip.io address to fall back to — set the server IP first.".to_string(),
            )
        })?;

    state
        .caddy
        .set_dashboard_domain(&fallback, &[])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write Caddyfile: {}", e)))?;
    if let Err(e) = state.caddy.persist_dashboard_domain_env(&fallback) {
        tracing::warn!("Failed to persist dashboard domain to ployer.env: {}", e);
    }
    repo.set_dashboard_domain("")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DashboardDomainResponse {
        url: format!("https://{}", fallback),
        hosts: state.caddy.dashboard_hosts(),
        domain: fallback,
        server_ip,
        dns: DnsResult {
            status: "skipped".to_string(),
            message: "Cloudflare records were left in place.".to_string(),
        },
    }))
}

#[derive(Debug, Serialize)]
struct ZonesResponse {
    zones: Vec<Zone>,
}

/// Domains the configured Cloudflare token can write DNS for. Lets the UI offer
/// a picker and tells the user up front whether their token covers the domain
/// they're about to set.
async fn list_cloudflare_zones(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ZonesResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let Some(token) = state.caddy.cf_token() else {
        return Ok(Json(ZonesResponse { zones: Vec::new() }));
    };

    let zones = CloudflareClient::new(token)
        .list_zones()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(ZonesResponse { zones }))
}

#[cfg(test)]
mod tests {
    use super::is_shared_ip_dns;

    #[test]
    fn shared_ip_dns_detection() {
        assert!(is_shared_ip_dns("3.144.143.144.nip.io"));
        assert!(is_shared_ip_dns("1.2.3.4.sslip.io"));
        assert!(!is_shared_ip_dns("ployer.example.com"));
        assert!(!is_shared_ip_dns("nip.io.example.com"));
    }
}
