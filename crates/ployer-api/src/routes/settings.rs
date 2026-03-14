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
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    allow_registration: bool,
}

async fn get_settings(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<SettingsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = SettingsRepository::new(state.db.clone());
    let allow_registration = repo
        .allow_registration()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SettingsResponse { allow_registration }))
}

async fn update_settings(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = SettingsRepository::new(state.db.clone());
    let value = if req.allow_registration { "true" } else { "false" };
    repo.set("allow_registration", value)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SettingsResponse {
        allow_registration: req.allow_registration,
    }))
}
