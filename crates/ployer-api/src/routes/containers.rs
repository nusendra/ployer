use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use ployer_docker::{ContainerConfig, ContainerInfo, ContainerStats, NetworkInfo, VolumeInfo};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_containers).post(create_container))
        .route("/:id", get(get_container).delete(remove_container))
        .route("/:id/start", post(start_container))
        .route("/:id/stop", post(stop_container))
        .route("/:id/restart", post(restart_container))
        .route("/:id/logs", get(get_container_logs))
        .route("/:id/stats", get(get_container_stats))
}

pub fn networks_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_networks).post(create_network))
        .route("/:id", get(get_network).delete(remove_network))
}

pub fn volumes_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_volumes).post(create_volume))
        .route("/:name", get(get_volume).delete(remove_volume))
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct ListContainersQuery {
    #[serde(default)]
    all: bool,
}

#[derive(Debug, Serialize)]
struct ListContainersResponse {
    containers: Vec<ContainerInfo>,
}

#[derive(Debug, Deserialize)]
struct CreateContainerRequest {
    image: String,
    name: Option<String>,
    env: Option<Vec<String>>,
    ports: Option<HashMap<String, String>>,
    volumes: Option<HashMap<String, String>>,
    network: Option<String>,
    cmd: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct ContainerResponse {
    container_id: String,
}

#[derive(Debug, Serialize)]
struct ContainerDetailsResponse {
    container: ContainerInfo,
}

#[derive(Debug, Deserialize)]
struct GetLogsQuery {
    #[serde(default = "default_tail")]
    tail: usize,
}

fn default_tail() -> usize {
    100
}

#[derive(Debug, Serialize)]
struct ContainerLogsResponse {
    logs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ContainerStatsResponse {
    stats: ContainerStats,
}

// ===== Handlers =====

async fn list_containers(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Query(query): Query<ListContainersQuery>,
) -> Result<Json<ListContainersResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let containers = docker
        .list_containers(query.all)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListContainersResponse { containers }))
}

async fn create_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CreateContainerRequest>,
) -> Result<(StatusCode, Json<ContainerResponse>), (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    // Validate input
    if req.image.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Image name is required".to_string()));
    }

    let config = ContainerConfig {
        image: req.image,
        name: req.name,
        env: req.env,
        ports: req.ports,
        volumes: req.volumes,
        network: req.network,
        cmd: req.cmd,
    };

    let container_id = docker
        .create_container(config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ContainerResponse { container_id }),
    ))
}

async fn get_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ContainerDetailsResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let inspect = docker
        .inspect_container(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    // Convert inspect response to ContainerInfo
    let container = ContainerInfo {
        id: inspect.id.unwrap_or_default(),
        name: inspect.name.unwrap_or_default().trim_start_matches('/').to_string(),
        image: inspect.config.and_then(|c| c.image).unwrap_or_default(),
        state: inspect.state.and_then(|s| s.status).unwrap_or_default().to_string(),
        status: "running".to_string(), // Simplified
        created: 0, // Would need to parse from inspect.created
        ports: vec![], // Would need to parse from inspect.network_settings
    };

    Ok(Json(ContainerDetailsResponse { container }))
}

async fn start_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .start_container(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("already started") {
                (StatusCode::CONFLICT, "Container already running".to_string())
            } else if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn stop_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .stop_container(&id, None)
        .await
        .map_err(|e| {
            if e.to_string().contains("not running") {
                (StatusCode::CONFLICT, "Container not running".to_string())
            } else if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn restart_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .restart_container(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn remove_container(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .remove_container(&id, true)
        .await
        .map_err(|e| {
            if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_container_logs(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<GetLogsQuery>,
) -> Result<Json<ContainerLogsResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let logs = docker
        .get_container_logs(&id, Some(query.tail))
        .await
        .map_err(|e| {
            if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(ContainerLogsResponse { logs }))
}

async fn get_container_stats(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ContainerStatsResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let stats = docker
        .get_container_stats(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("No such container") {
                (StatusCode::NOT_FOUND, "Container not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(ContainerStatsResponse { stats }))
}

// ===== Network Handlers =====

#[derive(Debug, Serialize)]
struct ListNetworksResponse {
    networks: Vec<NetworkInfo>,
}

#[derive(Debug, Deserialize)]
struct CreateNetworkRequest {
    name: String,
    #[serde(default = "default_driver")]
    driver: String,
}

fn default_driver() -> String {
    "bridge".to_string()
}

#[derive(Debug, Serialize)]
struct NetworkResponse {
    network_id: String,
}

#[derive(Debug, Serialize)]
struct NetworkDetailsResponse {
    network: NetworkInfo,
}

async fn list_networks(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ListNetworksResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let networks = docker
        .list_networks()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListNetworksResponse { networks }))
}

async fn create_network(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CreateNetworkRequest>,
) -> Result<(StatusCode, Json<NetworkResponse>), (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    // Validate input
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Network name is required".to_string()));
    }

    let network_id = docker
        .create_network(&req.name, &req.driver)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(NetworkResponse { network_id }),
    ))
}

async fn get_network(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<NetworkDetailsResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let network = docker
        .inspect_network(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Network not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(NetworkDetailsResponse { network }))
}

async fn remove_network(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .remove_network(&id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Network not found".to_string())
            } else if e.to_string().contains("in use") {
                (StatusCode::CONFLICT, "Network is in use by containers".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// ===== Volume Handlers =====

#[derive(Debug, Serialize)]
struct ListVolumesResponse {
    volumes: Vec<VolumeInfo>,
}

#[derive(Debug, Deserialize)]
struct CreateVolumeRequest {
    name: String,
}

#[derive(Debug, Serialize)]
struct VolumeResponse {
    volume: VolumeInfo,
}

async fn list_volumes(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ListVolumesResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let volumes = docker
        .list_volumes()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListVolumesResponse { volumes }))
}

async fn create_volume(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CreateVolumeRequest>,
) -> Result<(StatusCode, Json<VolumeResponse>), (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    // Validate input
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Volume name is required".to_string()));
    }

    let volume = docker
        .create_volume(&req.name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(VolumeResponse { volume }),
    ))
}

async fn get_volume(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<VolumeResponse>, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    let volume = docker
        .inspect_volume(&name)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Volume not found".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(Json(VolumeResponse { volume }))
}

async fn remove_volume(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate auth
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Check if Docker is available
    let docker = state
        .docker
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Docker not available".to_string()))?;

    docker
        .remove_volume(&name, false)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                (StatusCode::NOT_FOUND, "Volume not found".to_string())
            } else if e.to_string().contains("in use") {
                (StatusCode::CONFLICT, "Volume is in use by containers".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}
