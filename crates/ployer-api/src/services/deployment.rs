use anyhow::{anyhow, Result};
use ployer_core::models::{Application, Deployment, DeploymentStatus, WsEvent};
use ployer_db::repositories::{DeploymentRepository, DomainRepository};
use ployer_docker::{DockerClient, ContainerConfig};
use ployer_git::GitService;
use ployer_proxy::{CaddyClient, ReverseProxyConfig};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, warn};

pub struct DeploymentService {
    db: SqlitePool,
    docker: Arc<DockerClient>,
    #[allow(dead_code)]
    git: GitService,
    caddy: Option<Arc<CaddyClient>>,
    base_domain: String,
    ws_broadcast: broadcast::Sender<WsEvent>,
}

impl DeploymentService {
    pub fn new(
        db: SqlitePool,
        docker: Arc<DockerClient>,
        caddy: Option<Arc<CaddyClient>>,
        base_domain: String,
        ws_broadcast: broadcast::Sender<WsEvent>,
    ) -> Self {
        Self {
            db,
            docker,
            git: GitService::new(),
            caddy,
            base_domain,
            ws_broadcast,
        }
    }

    /// Trigger a new deployment for an application
    pub async fn deploy(
        &self,
        application: Application,
        private_key: Option<String>,
    ) -> Result<Deployment> {
        let deployment_repo = DeploymentRepository::new(self.db.clone());

        // Create deployment record
        let image_tag = format!("ployer-{}:{}", application.name, uuid::Uuid::new_v4());
        let deployment = deployment_repo
            .create(
                &application.id,
                &application.server_id,
                None, // commit_sha - will be updated after clone
                None, // commit_message - will be updated after clone
                &image_tag,
            )
            .await?;

        let deployment_id = deployment.id.clone();

        // Spawn deployment task in background
        let db = self.db.clone();
        let docker = self.docker.clone();
        let caddy = self.caddy.clone();
        let base_domain = self.base_domain.clone();
        let ws_broadcast = self.ws_broadcast.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_deployment(
                db,
                docker,
                caddy,
                base_domain,
                ws_broadcast,
                deployment_id,
                application,
                private_key,
                image_tag,
            )
            .await
            {
                error!("Deployment failed: {}", e);
            }
        });

        Ok(deployment)
    }

    /// Execute the full deployment pipeline
    async fn execute_deployment(
        db: SqlitePool,
        docker: Arc<DockerClient>,
        caddy: Option<Arc<CaddyClient>>,
        base_domain: String,
        ws_broadcast: broadcast::Sender<WsEvent>,
        deployment_id: String,
        application: Application,
        private_key: Option<String>,
        image_tag: String,
    ) -> Result<()> {
        let git = GitService::new();
        let deployment_repo = DeploymentRepository::new(db.clone());

        // Helper to broadcast logs and save to database
        let send_log = |msg: String| {
            let deployment_id = deployment_id.clone();
            let db = db.clone();
            let ws_broadcast = ws_broadcast.clone();
            async move {
                // Save to database
                let deployment_repo = DeploymentRepository::new(db);
                let _ = deployment_repo.append_log(&deployment_id, &msg).await;
                // Broadcast via WebSocket
                let _ = ws_broadcast.send(WsEvent::DeploymentLog {
                    deployment_id: deployment_id.clone(),
                    line: msg,
                });
            }
        };

        // Step 1: Clone git repository (if git_url is configured)
        let context_path = if let Some(git_url) = &application.git_url {
            deployment_repo.update_status(&deployment_id, DeploymentStatus::Cloning).await?;
            send_log(format!("Cloning repository: {}", git_url)).await;

            let clone_dir = PathBuf::from(format!("/tmp/ployer-builds/{}", deployment_id));
            tokio::fs::create_dir_all(&clone_dir).await?;

            git.clone_repo(
                git_url,
                &clone_dir,
                &application.git_branch,
                private_key.as_deref(),
            )?;

            // Get commit information
            let commit_info = git.get_latest_commit(&clone_dir)?;
            send_log(format!("Commit: {} - {}", commit_info.sha, commit_info.message)).await;

            // Update deployment with commit info
            sqlx::query!(
                "UPDATE deployments SET commit_sha = ?, commit_message = ? WHERE id = ?",
                commit_info.sha,
                commit_info.message,
                deployment_id
            )
            .execute(&db)
            .await?;

            clone_dir
        } else {
            return Err(anyhow!("Application has no git_url configured"));
        };

        // Step 2: Build Docker image
        deployment_repo.update_status(&deployment_id, DeploymentStatus::Building).await?;
        send_log("Building Docker image...".to_string()).await;

        let dockerfile_path = application.dockerfile_path.as_deref();
        let mut build_logs = docker.build_image(&context_path, dockerfile_path, &image_tag).await?;

        // Stream build logs
        while let Some(log_line) = build_logs.recv().await {
            send_log(log_line.trim().to_string()).await;
        }

        send_log("Build completed successfully".to_string()).await;

        // Step 3: Create and start new container
        deployment_repo.update_status(&deployment_id, DeploymentStatus::Deploying).await?;
        send_log("Creating container...".to_string()).await;

        let container_config = ContainerConfig {
            image: image_tag.clone(),
            name: Some(format!("{}-{}", application.name, deployment_id)),
            env: None, // TODO: Load from environment variables
            ports: application.port.map(|p| {
                let mut ports = HashMap::new();
                ports.insert(format!("{}/tcp", p), p.to_string());
                ports
            }),
            volumes: None,
            network: Some("bridge".to_string()),
            cmd: None,
        };

        let container_id = docker.create_container(container_config).await?;
        deployment_repo.set_container_id(&deployment_id, &container_id).await?;
        send_log(format!("Container created: {}", container_id)).await;

        docker.start_container(&container_id).await?;
        send_log("Container started".to_string()).await;

        // Step 4: Health check (simple wait for now)
        send_log("Waiting for health check...".to_string()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Step 5: Stop old container (rolling update)
        send_log("Performing rolling update...".to_string()).await;

        // Get the previous running deployment
        if let Ok(Some(prev_deployment)) = deployment_repo.get_latest_running(&application.id).await {
            if let Some(prev_container_id) = prev_deployment.container_id {
                if prev_container_id != container_id {
                    send_log(format!("Stopping old container: {}", prev_container_id)).await;

                    // Stop the old container (10 second timeout)
                    if let Err(e) = docker.stop_container(&prev_container_id, Some(10)).await {
                        warn!("Failed to stop old container {}: {}", prev_container_id, e);
                        send_log(format!("Warning: Failed to stop old container: {}", e)).await;
                    } else {
                        send_log(format!("Old container stopped: {}", prev_container_id)).await;
                    }

                    // Remove the old container (force=true to remove even if running)
                    if let Err(e) = docker.remove_container(&prev_container_id, true).await {
                        warn!("Failed to remove old container {}: {}", prev_container_id, e);
                        send_log(format!("Warning: Failed to remove old container: {}", e)).await;
                    } else {
                        send_log(format!("Old container removed: {}", prev_container_id)).await;
                    }

                    // Update old deployment status to rolled_back
                    let _ = deployment_repo.update_status(&prev_deployment.id, DeploymentStatus::RolledBack).await;
                }
            }
        }

        // Step 5.5: Create subdomain and configure Caddy
        // For MVP, skip actual Caddy configuration (would need Caddy running)
        // Just create the domain record
        send_log("Configuring domain...".to_string()).await;
        let subdomain = format!("{}.{}", application.name, base_domain);

        let domain_repo = DomainRepository::new(db.clone());
        // Check if subdomain already exists
        if domain_repo.find_by_domain(&subdomain).await.ok().flatten().is_none() {
            match domain_repo.create(&application.id, &subdomain, true).await {
                Ok(_) => {
                    send_log(format!("Subdomain created: {}", subdomain)).await;

                    // Configure Caddy if available
                    if let Some(ref caddy_client) = caddy {
                        if let Some(port) = application.port {
                            let upstream = format!("localhost:{}", port);
                            let caddy_config = ReverseProxyConfig {
                                domain: subdomain.clone(),
                                upstream,
                                enable_https: true,
                            };

                            if let Err(e) = caddy_client.add_route(caddy_config).await {
                                warn!("Failed to configure Caddy route: {}", e);
                                send_log(format!("Warning: Caddy configuration failed: {}", e)).await;
                            } else {
                                send_log(format!("Caddy configured: https://{}", subdomain)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create subdomain: {}", e);
                }
            }
        }

        // Step 6: Mark deployment as running
        deployment_repo.update_status(&deployment_id, DeploymentStatus::Running).await?;
        send_log("Deployment completed successfully!".to_string()).await;

        // Broadcast deployment status change
        let _ = ws_broadcast.send(WsEvent::DeploymentStatus {
            deployment_id: deployment_id.clone(),
            app_id: application.id.clone(),
            status: DeploymentStatus::Running,
        });

        // Clean up build directory
        let _ = tokio::fs::remove_dir_all(context_path).await;

        Ok(())
    }

    /// Cancel a running deployment
    pub async fn cancel_deployment(&self, deployment_id: &str) -> Result<bool> {
        let deployment_repo = DeploymentRepository::new(self.db.clone());
        let cancelled = deployment_repo.cancel(deployment_id).await?;

        if cancelled {
            // Get deployment to find app_id
            if let Some(deployment) = deployment_repo.find_by_id(deployment_id).await? {
                let _ = self.ws_broadcast.send(WsEvent::DeploymentStatus {
                    deployment_id: deployment_id.to_string(),
                    app_id: deployment.application_id,
                    status: DeploymentStatus::Cancelled,
                });
            }
        }

        Ok(cancelled)
    }
}
