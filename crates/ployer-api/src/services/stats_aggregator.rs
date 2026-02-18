use ployer_db::repositories::{ApplicationRepository, ContainerStatsRepository, DeploymentRepository};
use ployer_docker::DockerClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub fn spawn_stats_aggregator(db: SqlitePool, docker: Option<Arc<DockerClient>>) {
    tokio::spawn(async move {
        // Collect stats every 60 seconds
        let mut stats_interval = tokio::time::interval(Duration::from_secs(60));
        // Cleanup old stats every hour
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(3600));

        loop {
            tokio::select! {
                _ = stats_interval.tick() => {
                    if let Some(ref docker_client) = docker {
                        if let Err(e) = collect_container_stats(&db, docker_client).await {
                            warn!("Stats collection error: {}", e);
                        }
                    }
                }
                _ = cleanup_interval.tick() => {
                    if let Err(e) = cleanup_old_stats(&db).await {
                        warn!("Stats cleanup error: {}", e);
                    }
                }
            }
        }
    });

    info!("Container stats aggregator started (60s interval, 24h retention)");
}

async fn collect_container_stats(db: &SqlitePool, docker: &DockerClient) -> anyhow::Result<()> {
    let stats_repo = ContainerStatsRepository::new(db.clone());
    let deployment_repo = DeploymentRepository::new(db.clone());
    let app_repo = ApplicationRepository::new(db.clone());

    let applications = app_repo.list().await?;

    for app in applications {
        // Get the latest running deployment
        let deployment = match deployment_repo.get_latest_running(&app.id).await? {
            Some(d) => d,
            None => continue,
        };

        let container_id = match &deployment.container_id {
            Some(id) => id,
            None => continue,
        };

        // Get container stats from Docker (uses ployer-docker's ContainerStats type)
        match docker.get_container_stats(container_id).await {
            Ok(stats) => {
                // Network I/O in MB
                let network_rx_mb = stats.network_rx_bytes as f64 / 1_048_576.0;
                let network_tx_mb = stats.network_tx_bytes as f64 / 1_048_576.0;

                stats_repo
                    .record(
                        container_id,
                        Some(&app.id),
                        stats.cpu_usage,
                        stats.memory_usage_mb,
                        Some(stats.memory_limit_mb),
                        Some(network_rx_mb),
                        Some(network_tx_mb),
                    )
                    .await?;

                debug!(
                    "Recorded stats for app {}: CPU={:.2}%, Mem={:.2}MB",
                    app.name,
                    stats.cpu_usage,
                    stats.memory_usage_mb
                );
            }
            Err(e) => {
                debug!("Failed to get stats for container {}: {}", container_id, e);
            }
        }
    }

    Ok(())
}

async fn cleanup_old_stats(db: &SqlitePool) -> anyhow::Result<()> {
    let stats_repo = ContainerStatsRepository::new(db.clone());

    // Keep last 24 hours of stats
    let deleted = stats_repo.cleanup_old_stats(24).await?;

    if deleted > 0 {
        info!("Cleaned up {} old container stats records", deleted);
    }

    Ok(())
}
