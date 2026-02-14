use anyhow::Result;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    LogsOptions, RemoveContainerOptions, StartContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::image::BuildImageOptions;
use bollard::models::{ContainerInspectResponse, ContainerSummary, HostConfig, PortBinding};
use bollard::network::{CreateNetworkOptions, InspectNetworkOptions, ListNetworksOptions};
use bollard::volume::{CreateVolumeOptions, ListVolumesOptions, RemoveVolumeOptions};
use bollard::Docker;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tar::Builder;

pub struct DockerClient {
    client: Docker,
}

// Container configuration for creating new containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub name: Option<String>,
    pub env: Option<Vec<String>>,
    pub ports: Option<HashMap<String, String>>, // container_port -> host_port
    pub volumes: Option<HashMap<String, String>>, // host_path -> container_path
    pub network: Option<String>,
    pub cmd: Option<Vec<String>>,
}

// Container information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    pub ports: Vec<PortInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

// Container resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub memory_limit_mb: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub created: String,
    pub containers: Vec<String>,
}

// Volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
}

impl DockerClient {
    pub fn new(socket_path: &str) -> Result<Self> {
        let client = Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)?;
        info!("Docker client connected via {}", socket_path);
        Ok(Self { client })
    }

    pub fn inner(&self) -> &Docker {
        &self.client
    }

    pub async fn ping(&self) -> Result<bool> {
        match self.client.ping().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Build a Docker image from a context directory
    /// Returns a channel that streams build log lines
    pub async fn build_image(
        &self,
        context_path: &Path,
        dockerfile_path: Option<&str>,
        tag: &str,
    ) -> Result<mpsc::Receiver<String>> {
        info!("Building Docker image: {} from {:?}", tag, context_path);

        // Create a tar archive of the build context
        let tar_data = Self::create_build_context_tar(context_path)?;

        let options = BuildImageOptions {
            dockerfile: dockerfile_path.unwrap_or("Dockerfile").to_string(),
            t: tag.to_string(),
            rm: true, // Remove intermediate containers
            pull: true, // Always pull the latest base image
            ..Default::default()
        };

        // Clone the client to avoid borrowing self in the spawned task
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel(100);

        // Spawn a task to process the build stream
        tokio::spawn(async move {
            let mut stream = client.build_image(options, None, Some(tar_data.into()));
            while let Some(result) = stream.next().await {
                match result {
                    Ok(info) => {
                        // Extract log message from BuildInfo
                        if let Some(stream) = info.stream {
                            let _ = tx.send(stream).await;
                        } else if let Some(error) = info.error {
                            let _ = tx.send(format!("ERROR: {}", error)).await;
                        } else if let Some(status) = info.status {
                            let _ = tx.send(status).await;
                        }
                    }
                    Err(e) => {
                        warn!("Build stream error: {}", e);
                        let _ = tx.send(format!("ERROR: {}", e)).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Create a tar archive of the build context directory
    fn create_build_context_tar(context_path: &Path) -> Result<Vec<u8>> {
        let mut tar_data = Vec::new();
        {
            let mut tar = Builder::new(&mut tar_data);
            tar.append_dir_all(".", context_path)?;
            tar.finish()?;
        }
        Ok(tar_data)
    }

    // List containers
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>> {
        let options = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };

        let containers = self.client.list_containers(Some(options)).await?;

        Ok(containers.into_iter().map(|c| self.summary_to_info(c)).collect())
    }

    // Inspect container details
    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspectResponse> {
        let options = InspectContainerOptions { size: false };
        Ok(self.client.inspect_container(id, Some(options)).await?)
    }

    // Create a new container
    pub async fn create_container(&self, config: ContainerConfig) -> Result<String> {
        let name = config.name.clone();

        // Build port bindings
        let mut port_bindings = HashMap::new();
        if let Some(ports) = &config.ports {
            for (container_port, host_port) in ports {
                port_bindings.insert(
                    container_port.clone(),
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(host_port.clone()),
                    }]),
                );
            }
        }

        // Build volume bindings
        let binds = config.volumes.as_ref().map(|volumes| {
            volumes
                .iter()
                .map(|(host, container)| format!("{}:{}", host, container))
                .collect::<Vec<_>>()
        });

        let host_config = Some(HostConfig {
            port_bindings: Some(port_bindings),
            binds,
            network_mode: config.network,
            ..Default::default()
        });

        let container_config = Config {
            image: Some(config.image.clone()),
            env: config.env,
            cmd: config.cmd,
            host_config,
            ..Default::default()
        };

        let options = name.map(|n| CreateContainerOptions { name: n, ..Default::default() });

        let response = self.client.create_container(options, container_config).await?;

        Ok(response.id)
    }

    // Start a container
    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.client
            .start_container(id, None::<StartContainerOptions<String>>)
            .await?;
        Ok(())
    }

    // Stop a container
    pub async fn stop_container(&self, id: &str, timeout: Option<i64>) -> Result<()> {
        let options = StopContainerOptions { t: timeout.unwrap_or(10) };
        self.client.stop_container(id, Some(options)).await?;
        Ok(())
    }

    // Restart a container
    pub async fn restart_container(&self, id: &str) -> Result<()> {
        self.client.restart_container(id, None).await?;
        Ok(())
    }

    // Remove a container
    pub async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        let options = RemoveContainerOptions {
            force,
            v: true, // Remove volumes
            ..Default::default()
        };
        self.client.remove_container(id, Some(options)).await?;
        Ok(())
    }

    // Get container logs
    pub async fn get_container_logs(&self, id: &str, tail: Option<usize>) -> Result<Vec<String>> {
        use futures_util::StreamExt;

        let options = LogsOptions {
            stdout: true,
            stderr: true,
            tail: tail.unwrap_or(100).to_string(),
            ..Default::default()
        };

        let mut stream = self.client.logs(id, Some(options));
        let mut logs = Vec::new();

        while let Some(log) = stream.next().await {
            if let Ok(output) = log {
                logs.push(output.to_string());
            }
        }

        Ok(logs)
    }

    // Get container stats (one-shot)
    pub async fn get_container_stats(&self, id: &str) -> Result<ContainerStats> {
        use futures_util::StreamExt;

        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.client.stats(id, Some(options));

        if let Some(Ok(stats)) = stream.next().await {
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
                - stats.precpu_stats.cpu_usage.total_usage as f64;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
            let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;

            let cpu_usage = if system_delta > 0.0 && cpu_delta > 0.0 {
                (cpu_delta / system_delta) * num_cpus * 100.0
            } else {
                0.0
            };

            let memory_usage_mb = stats.memory_stats.usage.unwrap_or(0) as f64 / 1024.0 / 1024.0;
            let memory_limit_mb = stats.memory_stats.limit.unwrap_or(0) as f64 / 1024.0 / 1024.0;

            let (network_rx_bytes, network_tx_bytes) = stats
                .networks
                .as_ref()
                .and_then(|networks| networks.values().next())
                .map(|net| (net.rx_bytes, net.tx_bytes))
                .unwrap_or((0, 0));

            return Ok(ContainerStats {
                cpu_usage,
                memory_usage_mb,
                memory_limit_mb,
                network_rx_bytes,
                network_tx_bytes,
            });
        }

        Err(anyhow::anyhow!("Failed to get container stats"))
    }

    // ===== Network Management =====

    // List networks
    pub async fn list_networks(&self) -> Result<Vec<NetworkInfo>> {
        let networks = self.client.list_networks(None::<ListNetworksOptions<String>>).await?;

        Ok(networks
            .into_iter()
            .map(|n| NetworkInfo {
                id: n.id.unwrap_or_default(),
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
                scope: n.scope.unwrap_or_default(),
                created: n.created.unwrap_or_default(),
                containers: n
                    .containers
                    .map(|c| c.keys().cloned().collect())
                    .unwrap_or_default(),
            })
            .collect())
    }

    // Create network
    pub async fn create_network(&self, name: &str, driver: &str) -> Result<String> {
        let config = CreateNetworkOptions {
            name: name.to_string(),
            driver: driver.to_string(),
            ..Default::default()
        };

        let response = self.client.create_network(config).await?;

        Ok(response.id.unwrap_or_default())
    }

    // Inspect network
    pub async fn inspect_network(&self, id: &str) -> Result<NetworkInfo> {
        let options = InspectNetworkOptions {
            verbose: false,
            scope: "".to_string(),
        };

        let network = self.client.inspect_network(id, Some(options)).await?;

        Ok(NetworkInfo {
            id: network.id.unwrap_or_default(),
            name: network.name.unwrap_or_default(),
            driver: network.driver.unwrap_or_default(),
            scope: network.scope.unwrap_or_default(),
            created: network.created.unwrap_or_default(),
            containers: network
                .containers
                .map(|c| c.keys().cloned().collect())
                .unwrap_or_default(),
        })
    }

    // Remove network
    pub async fn remove_network(&self, id: &str) -> Result<()> {
        self.client.remove_network(id).await?;
        Ok(())
    }

    // ===== Volume Management =====

    // List volumes
    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        let response = self.client.list_volumes(None::<ListVolumesOptions<String>>).await?;

        Ok(response
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| VolumeInfo {
                name: v.name,
                driver: v.driver,
                mountpoint: v.mountpoint,
                created_at: v.created_at,
            })
            .collect())
    }

    // Create volume
    pub async fn create_volume(&self, name: &str) -> Result<VolumeInfo> {
        let config = CreateVolumeOptions {
            name: name.to_string(),
            driver: "local".to_string(),
            ..Default::default()
        };

        let volume = self.client.create_volume(config).await?;

        Ok(VolumeInfo {
            name: volume.name,
            driver: volume.driver,
            mountpoint: volume.mountpoint,
            created_at: volume.created_at,
        })
    }

    // Inspect volume
    pub async fn inspect_volume(&self, name: &str) -> Result<VolumeInfo> {
        let volume = self.client.inspect_volume(name).await?;

        Ok(VolumeInfo {
            name: volume.name,
            driver: volume.driver,
            mountpoint: volume.mountpoint,
            created_at: volume.created_at,
        })
    }

    // Remove volume
    pub async fn remove_volume(&self, name: &str, force: bool) -> Result<()> {
        let options = RemoveVolumeOptions { force };
        self.client.remove_volume(name, Some(options)).await?;
        Ok(())
    }

    // ===== Helper Methods =====

    // Helper to convert bollard's ContainerSummary to our ContainerInfo
    fn summary_to_info(&self, summary: ContainerSummary) -> ContainerInfo {
        let ports = summary
            .ports
            .unwrap_or_default()
            .into_iter()
            .map(|p| PortInfo {
                container_port: p.private_port,
                host_port: p.public_port,
                protocol: p.typ.map(|t| t.to_string()).unwrap_or_else(|| "tcp".to_string()),
            })
            .collect();

        ContainerInfo {
            id: summary.id.unwrap_or_default(),
            name: summary.names.unwrap_or_default().first().unwrap_or(&String::new()).trim_start_matches('/').to_string(),
            image: summary.image.unwrap_or_default(),
            state: summary.state.unwrap_or_default(),
            status: summary.status.unwrap_or_default(),
            created: summary.created.unwrap_or(0),
            ports,
        }
    }
}
