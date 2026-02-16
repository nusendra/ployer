use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ployer_core::models::{HealthCheckStatus, ContainerStats};
use serde::{Deserialize, Serialize};

use crate::app_state::SharedState;
use crate::auth::AuthUser;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/applications/:app_id/health-check",
            post(configure_health_check).get(get_health_check),
        )
        .route(
            "/applications/:app_id/health-check/results",
            get(get_health_check_results),
        )
        .route(
            "/applications/:app_id/stats",
            get(get_application_stats),
        )
}

#[derive(Debug, Deserialize)]
struct ConfigureHealthCheckRequest {
    path: String,
    interval_seconds: i32,
    timeout_seconds: i32,
    healthy_threshold: i32,
    unhealthy_threshold: i32,
}

#[derive(Debug, Serialize)]
struct HealthCheckResponse {
    id: String,
    application_id: String,
    path: String,
    interval_seconds: i32,
    timeout_seconds: i32,
    healthy_threshold: i32,
    unhealthy_threshold: i32,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct HealthCheckResultResponse {
    id: String,
    container_id: String,
    status: HealthCheckStatus,
    response_time_ms: Option<i32>,
    status_code: Option<i32>,
    error_message: Option<String>,
    checked_at: String,
}

#[derive(Debug, Deserialize)]
struct StatsQuery {
    hours: Option<i64>,
}

/// Configure health check for an application
async fn configure_health_check(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
    Json(req): Json<ConfigureHealthCheckRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health_repo = ployer_db::repositories::HealthCheckRepository::new(state.db.clone());
    let app_repo = ployer_db::repositories::ApplicationRepository::new(state.db.clone());

    // Verify application exists
    app_repo
        .get(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Upsert health check configuration
    let health_check = health_repo
        .upsert(
            &app_id,
            &req.path,
            req.interval_seconds,
            req.timeout_seconds,
            req.healthy_threshold,
            req.unhealthy_threshold,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(HealthCheckResponse {
        id: health_check.id,
        application_id: health_check.application_id,
        path: health_check.path,
        interval_seconds: health_check.interval_seconds,
        timeout_seconds: health_check.timeout_seconds,
        healthy_threshold: health_check.healthy_threshold,
        unhealthy_threshold: health_check.unhealthy_threshold,
        created_at: health_check.created_at.to_rfc3339(),
    }))
}

/// Get health check configuration for an application
async fn get_health_check(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health_repo = ployer_db::repositories::HealthCheckRepository::new(state.db.clone());

    let health_check = health_repo
        .get(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Health check not configured".to_string()))?;

    Ok(Json(HealthCheckResponse {
        id: health_check.id,
        application_id: health_check.application_id,
        path: health_check.path,
        interval_seconds: health_check.interval_seconds,
        timeout_seconds: health_check.timeout_seconds,
        healthy_threshold: health_check.healthy_threshold,
        unhealthy_threshold: health_check.unhealthy_threshold,
        created_at: health_check.created_at.to_rfc3339(),
    }))
}

/// Get health check results for an application
async fn get_health_check_results(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health_repo = ployer_db::repositories::HealthCheckRepository::new(state.db.clone());

    let results = health_repo
        .get_recent_results(&app_id, 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<HealthCheckResultResponse> = results
        .into_iter()
        .map(|r| HealthCheckResultResponse {
            id: r.id,
            container_id: r.container_id,
            status: r.status,
            response_time_ms: r.response_time_ms,
            status_code: r.status_code,
            error_message: r.error_message,
            checked_at: r.checked_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(response))
}

/// Get container stats for an application
async fn get_application_stats(
    _auth: AuthUser,
    State(state): State<SharedState>,
    Path(app_id): Path<String>,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let stats_repo = ployer_db::repositories::ContainerStatsRepository::new(state.db.clone());

    let hours = query.hours.unwrap_or(1); // Default to last 1 hour

    let stats = stats_repo
        .get_app_stats(&app_id, hours)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert to response format
    let response: Vec<serde_json::Value> = stats
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "container_id": s.container_id,
                "cpu_percent": s.cpu_percent,
                "memory_mb": s.memory_mb,
                "memory_limit_mb": s.memory_limit_mb,
                "network_rx_mb": s.network_rx_mb,
                "network_tx_mb": s.network_tx_mb,
                "recorded_at": s.recorded_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(response))
}
