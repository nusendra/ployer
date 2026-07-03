use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ployer_db::repositories::SettingsRepository;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_settings).put(update_settings))
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
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    #[serde(default)]
    allow_registration: Option<bool>,
    /// Set/replace the Cloudflare API token. An empty string clears it. Omit to
    /// leave it unchanged.
    #[serde(default)]
    cf_api_token: Option<String>,
}

async fn current_settings(state: &SharedState) -> Result<SettingsResponse, (StatusCode, String)> {
    let repo = SettingsRepository::new(state.db.clone());
    let allow_registration = repo
        .allow_registration()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(SettingsResponse {
        allow_registration,
        cf_api_token_set: state.caddy.cf_token_is_set(),
        cloudflare_plugin_available: state.caddy.cloudflare_plugin_available(),
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

    Ok(Json(current_settings(&state).await?))
}
