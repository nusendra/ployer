use ployer_core::models::{ServerStatus, WsEvent};
use ployer_db::repositories::ServerRepository;
use ployer_server::ServerManager;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub fn spawn_health_monitor(db: SqlitePool, ws_broadcast: broadcast::Sender<WsEvent>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            if let Err(e) = check_servers(&db, &ws_broadcast).await {
                warn!("Health check error: {}", e);
            }
        }
    });

    info!("Health monitor started (30s interval)");
}

async fn check_servers(db: &SqlitePool, ws_broadcast: &broadcast::Sender<WsEvent>) -> anyhow::Result<()> {
    let repo = ServerRepository::new(db.clone());
    let servers = repo.list().await?;

    for server in servers {
        let old_status = server.status.clone();

        let new_status = if server.is_local {
            // Local server is always online if Ployer is running
            ServerStatus::Online
        } else {
            // Test remote server connectivity
            match ServerManager::test_ssh_connection(
                &server.host,
                server.port,
                &server.username,
                server.ssh_key_encrypted.as_deref(),
            )
            .await
            {
                Ok(reachable) => {
                    if reachable {
                        ServerStatus::Online
                    } else {
                        ServerStatus::Offline
                    }
                }
                Err(_) => ServerStatus::Offline,
            }
        };

        // Update if status changed
        if old_status != new_status {
            info!(
                "Server {} ({}): {} -> {}",
                server.name,
                server.id,
                old_status.as_str(),
                new_status.as_str()
            );

            repo.update_status(&server.id, new_status.clone(), chrono::Utc::now())
                .await?;

            // Broadcast WebSocket event
            let _ = ws_broadcast.send(WsEvent::ServerHealth {
                server_id: server.id.clone(),
                status: new_status,
            });
        }
    }

    Ok(())
}
