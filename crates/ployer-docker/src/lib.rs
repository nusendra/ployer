use anyhow::Result;
use bollard::Docker;
use tracing::info;

pub struct DockerClient {
    client: Docker,
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
}
