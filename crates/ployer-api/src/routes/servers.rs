use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ployer_core::models::{Server, ServerStatus};
use ployer_db::repositories::ServerRepository;
use ployer_server::ServerManager;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_servers).post(create_server))
        .route("/:id", get(get_server).put(update_server).delete(delete_server))
        .route("/:id/resources", get(get_server_resources))
        .route("/:id/validate", post(validate_server))
}

#[derive(Debug, Serialize)]
struct ListServersResponse {
    servers: Vec<Server>,
}

async fn list_servers(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ListServersResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());
    let servers = repo.list().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListServersResponse { servers }))
}

#[derive(Debug, Deserialize)]
struct CreateServerRequest {
    name: String,
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_username")]
    username: String,
    ssh_key: Option<String>,
    #[serde(default)]
    is_local: bool,
}

fn default_port() -> u16 { 22 }
fn default_username() -> String { "root".to_string() }

#[derive(Debug, Serialize)]
struct ServerResponse {
    server: Server,
}

async fn create_server(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerResponse>), (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Validate input
    if req.name.trim().is_empty() || req.host.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name and host are required".to_string()));
    }

    let repo = ServerRepository::new(state.db.clone());
    let server = repo.create(
        &req.name,
        &req.host,
        req.port,
        &req.username,
        req.ssh_key.as_deref(),
        req.is_local,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(ServerResponse { server })))
}

async fn get_server(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ServerResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());
    let server = repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    Ok(Json(ServerResponse { server }))
}

#[derive(Debug, Deserialize)]
struct UpdateServerRequest {
    name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    ssh_key: Option<String>,
    is_local: Option<bool>,
}

async fn update_server(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<ServerResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());

    // Get existing server
    let existing = repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    // Use new values or keep existing
    let name = req.name.as_deref().unwrap_or(&existing.name);
    let host = req.host.as_deref().unwrap_or(&existing.host);
    let port = req.port.unwrap_or(existing.port);
    let username = req.username.as_deref().unwrap_or(&existing.username);
    let ssh_key = req.ssh_key.as_deref().or(existing.ssh_key_encrypted.as_deref());
    let is_local = req.is_local.unwrap_or(existing.is_local);

    let server = repo.update(&id, name, host, port, username, ssh_key, is_local)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ServerResponse { server }))
}

async fn delete_server(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());

    // Check if server exists
    repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    repo.delete(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct ServerResourcesResponse {
    stats: ployer_server::LocalStats,
}

async fn get_server_resources(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ServerResourcesResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());
    let server = repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    // Only local servers supported for now
    if !server.is_local {
        return Err((StatusCode::NOT_IMPLEMENTED, "Resource stats only available for local servers".to_string()));
    }

    let mut manager = ServerManager::new();
    let stats = manager.local_stats();

    Ok(Json(ServerResourcesResponse { stats }))
}

#[derive(Debug, Serialize)]
struct ValidateServerResponse {
    reachable: bool,
    status: String,
}

async fn validate_server(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ValidateServerResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ServerRepository::new(state.db.clone());
    let server = repo.find_by_id(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Server not found".to_string()))?;

    // Test connection
    let reachable = ServerManager::test_ssh_connection(
        &server.host,
        server.port,
        &server.username,
        server.ssh_key_encrypted.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update server status
    let new_status = if reachable {
        ServerStatus::Online
    } else {
        ServerStatus::Offline
    };

    repo.update_status(&id, new_status.clone(), chrono::Utc::now())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ValidateServerResponse {
        reachable,
        status: new_status.as_str().to_string(),
    }))
}
