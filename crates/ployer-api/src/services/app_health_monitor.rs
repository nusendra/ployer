use ployer_core::models::{HealthCheckStatus, WsEvent};
use ployer_db::repositories::{ApplicationRepository, DeploymentRepository, HealthCheckRepository};
use ployer_docker::DockerClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

pub fn spawn_app_health_monitor(
    db: SqlitePool,
    docker: Option<Arc<DockerClient>>,
    ws_broadcast: broadcast::Sender<WsEvent>,
) {
    tokio::spawn(async move {
        // Check health every 15 seconds
        let mut interval = tokio::time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            if let Some(ref docker_client) = docker {
                if let Err(e) = check_application_health(&db, docker_client, &ws_broadcast).await {
                    warn!("Application health check error: {}", e);
                }
            }
        }
    });

    info!("Application health monitor started (15s interval)");
}

async fn check_application_health(
    db: &SqlitePool,
    docker: &DockerClient,
    ws_broadcast: &broadcast::Sender<WsEvent>,
) -> anyhow::Result<()> {
    let health_repo = HealthCheckRepository::new(db.clone());
    let app_repo = ApplicationRepository::new(db.clone());
    let deployment_repo = DeploymentRepository::new(db.clone());

    let health_checks = health_repo.list().await?;

    for health_check in health_checks {
        // Get the application
        let app = match app_repo.find_by_id(&health_check.application_id).await? {
            Some(app) => app,
            None => continue,
        };

        // Get the latest running deployment
        let deployment = match deployment_repo.get_latest_running(&app.id).await? {
            Some(deployment) => deployment,
            None => {
                debug!("No running deployment for app {}", app.name);
                continue;
            }
        };

        let container_id = match &deployment.container_id {
            Some(id) => id,
            None => continue,
        };

        // Get the old status
        let old_status = health_repo
            .get_latest_status(&app.id)
            .await?
            .unwrap_or(HealthCheckStatus::Unknown);

        // Perform health check
        let (new_status, response_time_ms, status_code, error_message) =
            perform_health_check(docker, container_id, &health_check.path, health_check.timeout_seconds).await;

        // Record the result
        health_repo
            .record_result(
                &app.id,
                container_id,
                new_status.clone(),
                response_time_ms,
                status_code,
                error_message.as_deref(),
            )
            .await?;

        // Broadcast WebSocket event if status changed
        if old_status != new_status {
            info!(
                "App {} health: {} -> {}",
                app.name,
                old_status.as_str(),
                new_status.as_str()
            );

            let _ = ws_broadcast.send(WsEvent::AppHealth {
                app_id: app.id.clone(),
                status: new_status.clone(),
            });
        }

        // Auto-restart logic: check if we need to restart the container
        if new_status == HealthCheckStatus::Unhealthy {
            // Get recent results to count consecutive failures
            let recent_results = health_repo
                .get_recent_results(&app.id, health_check.unhealthy_threshold as i64)
                .await?;

            // Count consecutive unhealthy checks
            let consecutive_unhealthy = recent_results
                .iter()
                .take_while(|r| r.status == HealthCheckStatus::Unhealthy)
                .count();

            // If threshold exceeded, restart container
            if consecutive_unhealthy >= health_check.unhealthy_threshold as usize {
                warn!(
                    "App {} has {} consecutive unhealthy checks, restarting container {}",
                    app.name, consecutive_unhealthy, container_id
                );

                match docker.restart_container(container_id).await {
                    Ok(_) => {
                        info!("Successfully restarted container {} for app {}", container_id, app.name);

                        // Broadcast restart event
                        let _ = ws_broadcast.send(WsEvent::AppHealth {
                            app_id: app.id.clone(),
                            status: HealthCheckStatus::Unknown,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "Failed to restart container {} for app {}: {}",
                            container_id, app.name, e
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

async fn perform_health_check(
    docker: &DockerClient,
    container_id: &str,
    path: &str,
    timeout_seconds: i32,
) -> (HealthCheckStatus, Option<i32>, Option<i32>, Option<String>) {
    // Get container info to find the port
    let container = match docker.inspect_container(container_id).await {
        Ok(container) => container,
        Err(e) => {
            return (
                HealthCheckStatus::Unknown,
                None,
                None,
                Some(format!("Failed to inspect container: {}", e)),
            );
        }
    };

    // Try to find the exposed port
    let port = container
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .and_then(|ports| {
            // Get the first exposed port mapping
            ports.iter().find_map(|(_, bindings)| {
                bindings.as_ref()?.first()?.host_port.as_ref()
            })
        });

    let port = match port {
        Some(p) => p,
        None => {
            return (
                HealthCheckStatus::Unknown,
                None,
                None,
                Some("No port mapping found for container".to_string()),
            );
        }
    };

    // Make HTTP request to health check endpoint
    let url = format!("http://localhost:{}{}", port, path);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds as u64))
        .build()
        .unwrap();

    let start = std::time::Instant::now();

    match client.get(&url).send().await {
        Ok(response) => {
            let response_time = start.elapsed().as_millis() as i32;
            let status_code = response.status().as_u16() as i32;

            let status = if response.status().is_success() {
                HealthCheckStatus::Healthy
            } else {
                HealthCheckStatus::Unhealthy
            };

            (status, Some(response_time), Some(status_code), None)
        }
        Err(e) => {
            let response_time = start.elapsed().as_millis() as i32;
            (
                HealthCheckStatus::Unhealthy,
                Some(response_time),
                None,
                Some(e.to_string()),
            )
        }
    }
}
