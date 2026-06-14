use anyhow::{anyhow, Result};
use ployer_core::models::{AppStatus, Application, Deployment, DeploymentStatus, WsEvent};
use ployer_db::repositories::{ApplicationRepository, DeploymentRepository, DomainRepository, EnvVarRepository};
use ployer_docker::{DockerClient, ContainerConfig};
use ployer_git::GitService;
use ployer_proxy::CaddyClient;
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
    secret_key: [u8; 32],
    ws_broadcast: broadcast::Sender<WsEvent>,
}

impl DeploymentService {
    pub fn new(
        db: SqlitePool,
        docker: Arc<DockerClient>,
        caddy: Option<Arc<CaddyClient>>,
        base_domain: String,
        secret_key: [u8; 32],
        ws_broadcast: broadcast::Sender<WsEvent>,
    ) -> Self {
        Self {
            db,
            docker,
            git: GitService::new(),
            caddy,
            base_domain,
            secret_key,
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

        // Fixed image tag per app — always overwrite :latest
        let image_tag = format!("ployer-{}:latest", application.name);
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
        let secret_key = self.secret_key;
        let ws_broadcast = self.ws_broadcast.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_deployment(
                db.clone(),
                docker,
                caddy,
                base_domain,
                secret_key,
                ws_broadcast.clone(),
                deployment_id.clone(),
                application.clone(),
                private_key,
                image_tag,
            )
            .await
            {
                error!("Deployment failed: {}", e);
                let repo = DeploymentRepository::new(db.clone());
                let _ = repo.update_status(&deployment_id, DeploymentStatus::Failed).await;
                let _ = repo.append_log(&deployment_id, &format!("ERROR: {}", e)).await;
                let _ = ApplicationRepository::new(db)
                    .update_status(&application.id, AppStatus::Failed).await;
                let _ = ws_broadcast.send(WsEvent::DeploymentStatus {
                    deployment_id,
                    app_id: application.id,
                    status: DeploymentStatus::Failed,
                });
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
        secret_key: [u8; 32],
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

        // Step 2.5: Auto-detect port from EXPOSE if not set by user
        let effective_port: Option<u16> = if application.port.is_some() {
            application.port
        } else {
            match docker.get_image_exposed_port(&image_tag).await {
                Ok(Some(port)) => {
                    send_log(format!("Auto-detected port {} from Dockerfile EXPOSE", port)).await;
                    // Persist so the UI shows it and future deploys use it
                    let _ = ApplicationRepository::new(db.clone())
                        .update_port(&application.id, port).await;
                    Some(port)
                }
                Ok(None) => {
                    send_log("Warning: no EXPOSE in Dockerfile and no port configured — subdomain routing will be skipped".to_string()).await;
                    None
                }
                Err(e) => {
                    send_log(format!("Warning: could not detect port from image: {}", e)).await;
                    None
                }
            }
        };

        // Step 3: Remove any existing containers for this app (avoids port conflicts)
        deployment_repo.update_status(&deployment_id, DeploymentStatus::Deploying).await?;

        let container_name = format!("ployer-{}", application.name);

        // Force-remove by DB-tracked container ID (covers any naming scheme)
        if let Ok(Some(prev)) = deployment_repo.get_latest_running(&application.id).await {
            if let Some(prev_container_id) = &prev.container_id {
                send_log(format!("Removing previous container ({})...", &prev_container_id[..12])).await;
                match docker.remove_container(prev_container_id, true).await {
                    Ok(_) => send_log("Previous container removed".to_string()).await,
                    Err(e) => send_log(format!("Warning: could not remove previous container: {}", e)).await,
                }
            }
            let _ = deployment_repo.update_status(&prev.id, DeploymentStatus::RolledBack).await;
        }

        // Also force-remove by fixed name (catches untracked containers with the same name)
        match docker.remove_container(&container_name, true).await {
            Ok(_) => send_log(format!("Removed existing container '{}'", container_name)).await,
            Err(_) => {} // doesn't exist — that's fine
        }

        // Kill any remaining containers still bound to the app's port
        if let Some(port) = effective_port {
            match docker.remove_containers_by_port(port).await {
                Ok(removed) if !removed.is_empty() => {
                    send_log(format!("Freed port {} (removed: {})", port, removed.join(", "))).await;
                }
                _ => {}
            }
        }

        // Step 4: Create and start new container with fixed name
        send_log("Creating container...".to_string()).await;

        // Load and decrypt environment variables for this app
        let env_vars = {
            let env_repo = EnvVarRepository::new(db.clone());
            match env_repo.list_by_application(&application.id).await {
                Ok(vars) if !vars.is_empty() => {
                    let mut kv_pairs = Vec::new();
                    for var in &vars {
                        match ployer_core::crypto::decrypt(&var.value_encrypted, &secret_key) {
                            Ok(val) => kv_pairs.push(format!("{}={}", var.key, val)),
                            Err(e) => warn!("Failed to decrypt env var {}: {}", var.key, e),
                        }
                    }
                    send_log(format!("Loaded {} environment variable(s)", kv_pairs.len())).await;
                    Some(kv_pairs)
                }
                Ok(_) => None,
                Err(e) => {
                    warn!("Failed to load env vars: {}", e);
                    None
                }
            }
        };

        let container_config = ContainerConfig {
            image: image_tag.clone(),
            name: Some(container_name.clone()),
            env: env_vars,
            ports: effective_port.map(|p| {
                let mut ports = HashMap::new();
                ports.insert(format!("{}/tcp", p), p.to_string());
                ports
            }),
            volumes: None,
            network: Some("bridge".to_string()),
            cmd: None,
            cpu_limit: application.cpu_limit,
            memory_limit: application.memory_limit,
            restart: None,
        };

        let container_id = docker.create_container(container_config).await?;
        deployment_repo.set_container_id(&deployment_id, &container_id).await?;
        send_log(format!("Container '{}' created", container_name)).await;

        docker.start_container(&container_id).await?;
        send_log(format!("Container '{}' started", container_name)).await;

        // Step 5: Health check (simple wait)
        send_log("Waiting for health check...".to_string()).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Step 5.5: Create subdomain and configure Caddy
        // For MVP, skip actual Caddy configuration (would need Caddy running)
        // Just create the domain record
        send_log("Configuring domain...".to_string()).await;
        let subdomain = format!("{}.{}", application.name, base_domain);

        let domain_repo = DomainRepository::new(db.clone());

        // Always (re)write the Caddy route on every deploy — apps.caddy may have been
        // wiped on reinstall even if the domain record already exists in the DB.
        if let Some(ref caddy_client) = caddy {
            if let Some(port) = effective_port {
                let upstream = format!("localhost:{}", port);
                if let Err(e) = caddy_client.persist_route(&subdomain, &upstream) {
                    warn!("Failed to persist Caddy route: {}", e);
                    send_log(format!("Warning: Caddy route persistence failed: {}", e)).await;
                } else {
                    send_log(format!("Caddy configured: http://{}", subdomain)).await;
                }
            }
        }

        // Create domain record if it doesn't already exist
        if domain_repo.find_by_domain(&subdomain).await.ok().flatten().is_none() {
            if let Err(e) = domain_repo.create(&application.id, &subdomain, true).await {
                warn!("Failed to create subdomain record: {}", e);
            } else {
                send_log(format!("Subdomain created: {}", subdomain)).await;
            }
        }

        // Step 6: Mark deployment as running and update application status
        deployment_repo.update_status(&deployment_id, DeploymentStatus::Running).await?;
        ApplicationRepository::new(db.clone())
            .update_status(&application.id, AppStatus::Running).await?;
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

    /// Trigger a deployment for an application installed from a service
    /// template. Uses the compose YAML stored on the application row.
    pub async fn deploy_compose(&self, application: Application) -> Result<Deployment> {
        let compose_yaml = application
            .compose_content
            .clone()
            .ok_or_else(|| anyhow!("application has no compose_content"))?;

        let deployment_repo = DeploymentRepository::new(self.db.clone());
        let image_tag = format!("ployer-{}:compose", application.name);
        let deployment = deployment_repo
            .create(
                &application.id,
                &application.server_id,
                None,
                None,
                &image_tag,
            )
            .await?;

        let deployment_id = deployment.id.clone();
        let db = self.db.clone();
        let docker = self.docker.clone();
        let ws_broadcast = self.ws_broadcast.clone();
        let app = application.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_compose_deployment(
                db.clone(),
                docker,
                ws_broadcast.clone(),
                deployment_id.clone(),
                app.clone(),
                compose_yaml,
            )
            .await
            {
                error!("Compose deployment failed: {}", e);
                let repo = DeploymentRepository::new(db.clone());
                let _ = repo.update_status(&deployment_id, DeploymentStatus::Failed).await;
                let _ = repo.append_log(&deployment_id, &format!("ERROR: {}", e)).await;
                let _ = ApplicationRepository::new(db)
                    .update_status(&app.id, AppStatus::Failed)
                    .await;
                let _ = ws_broadcast.send(WsEvent::DeploymentStatus {
                    deployment_id,
                    app_id: app.id,
                    status: DeploymentStatus::Failed,
                });
            }
        });

        Ok(deployment)
    }

    async fn execute_compose_deployment(
        db: SqlitePool,
        docker: Arc<DockerClient>,
        ws_broadcast: broadcast::Sender<WsEvent>,
        deployment_id: String,
        application: Application,
        compose_yaml: String,
    ) -> Result<()> {
        let deployment_repo = DeploymentRepository::new(db.clone());

        let send_log = |msg: String| {
            let deployment_id = deployment_id.clone();
            let db = db.clone();
            let ws_broadcast = ws_broadcast.clone();
            async move {
                let repo = DeploymentRepository::new(db);
                let _ = repo.append_log(&deployment_id, &msg).await;
                let _ = ws_broadcast.send(WsEvent::DeploymentLog {
                    deployment_id: deployment_id.clone(),
                    line: msg,
                });
            }
        };

        deployment_repo
            .update_status(&deployment_id, DeploymentStatus::Deploying)
            .await?;

        let compose = ployer_templates::compose::parse(&compose_yaml)
            .map_err(|e| anyhow!("parse compose: {e}"))?;

        if compose.services.is_empty() {
            return Err(anyhow!("compose file has no services"));
        }

        // Step 1: ensure the shared 'ployer' network exists.
        send_log("Ensuring ployer network...".to_string()).await;
        docker.ensure_network("ployer").await?;

        for (service_name, service) in &compose.services {
            send_log(format!("--- Service: {} ---", service_name)).await;

            // Step 2: pull the image.
            send_log(format!("Pulling {}...", service.image)).await;
            let mut pull_logs = docker.pull_image(&service.image).await?;
            while let Some(line) = pull_logs.recv().await {
                send_log(line).await;
            }

            // Step 3: ensure named volumes exist and collect bind specs.
            let mut volume_binds = HashMap::new();
            for vol_spec in &service.volumes {
                let (host, container) = ployer_templates::compose::split_volume(vol_spec)
                    .ok_or_else(|| anyhow!("invalid volume spec: {vol_spec}"))?;
                if !host.starts_with('/') && !host.starts_with('.') {
                    docker.ensure_volume(&host).await?;
                    send_log(format!("Volume ready: {}", host)).await;
                }
                volume_binds.insert(host, container);
            }

            // Step 4: remove any existing container with the same name.
            let container_name = format!("ployer-{}-{}", application.name, service_name);
            if docker.remove_container(&container_name, true).await.is_ok() {
                send_log(format!("Removed existing container '{}'", container_name)).await;
            }

            // Step 5: prepare env + ports.
            let env = service
                .environment
                .as_ref()
                .map(|e| e.to_pairs())
                .filter(|v| !v.is_empty());

            let mut ports_map = HashMap::new();
            for port_spec in &service.ports {
                if let Some((host, container)) = ployer_templates::compose::split_port(port_spec) {
                    ports_map.insert(format!("{}/tcp", container), host);
                }
            }
            let ports = if ports_map.is_empty() { None } else { Some(ports_map) };

            // Step 6: create + start.
            let config = ContainerConfig {
                image: service.image.clone(),
                name: Some(container_name.clone()),
                env,
                ports,
                volumes: if volume_binds.is_empty() { None } else { Some(volume_binds) },
                network: Some("ployer".to_string()),
                cmd: None,
                cpu_limit: application.cpu_limit,
                memory_limit: application.memory_limit,
                restart: None,
            };

            let container_id = docker.create_container(config).await?;
            deployment_repo
                .set_container_id(&deployment_id, &container_id)
                .await?;
            send_log(format!("Container '{}' created", container_name)).await;

            docker.start_container(&container_id).await?;
            send_log(format!("Container '{}' started", container_name)).await;
        }

        deployment_repo
            .update_status(&deployment_id, DeploymentStatus::Running)
            .await?;
        ApplicationRepository::new(db.clone())
            .update_status(&application.id, AppStatus::Running)
            .await?;

        send_log("Service installed successfully".to_string()).await;

        let _ = ws_broadcast.send(WsEvent::DeploymentStatus {
            deployment_id,
            app_id: application.id,
            status: DeploymentStatus::Running,
        });

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
