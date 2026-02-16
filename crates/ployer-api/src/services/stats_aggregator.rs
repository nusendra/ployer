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

    // Get all applications
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

        // Get container stats from Docker
        match docker.get_container_stats(container_id).await {
            Ok(stats) => {
                // Extract CPU percentage
                let cpu_percent = calculate_cpu_percent(&stats);

                // Extract memory usage
                let memory_mb = stats.memory_stats.usage.unwrap_or(0) as f64 / 1_048_576.0; // bytes to MB
                let memory_limit_mb = stats.memory_stats.limit.map(|l| l as f64 / 1_048_576.0);

                // Extract network I/O
                let (network_rx_mb, network_tx_mb) = calculate_network_io(&stats);

                // Record stats
                stats_repo
                    .record(
                        container_id,
                        Some(&app.id),
                        cpu_percent,
                        memory_mb,
                        memory_limit_mb,
                        network_rx_mb,
                        network_tx_mb,
                    )
                    .await?;

                debug!(
                    "Recorded stats for container {}: CPU={:.2}%, Mem={:.2}MB",
                    &container_id[..12],
                    cpu_percent,
                    memory_mb
                );
            }
            Err(e) => {
                debug!("Failed to get stats for container {}: {}", container_id, e);
            }
        }
    }

    Ok(())
}

fn calculate_cpu_percent(stats: &bollard::container::Stats) -> f64 {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
        - stats.precpu_stats.cpu_usage.total_usage as f64;

    let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;

    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;

    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    }
}

fn calculate_network_io(stats: &bollard::container::Stats) -> (Option<f64>, Option<f64>) {
    if let Some(networks) = &stats.networks {
        let total_rx: u64 = networks.values().map(|n| n.rx_bytes).sum();
        let total_tx: u64 = networks.values().map(|n| n.tx_bytes).sum();

        (
            Some(total_rx as f64 / 1_048_576.0), // bytes to MB
            Some(total_tx as f64 / 1_048_576.0),
        )
    } else {
        (None, None)
    }
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
