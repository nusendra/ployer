use anyhow::Result;
use sysinfo::System;
use std::time::Duration;
use tracing::info;

pub struct ServerManager {
    system: System,
}

impl ServerManager {
    pub fn new() -> Self {
        info!("Server manager initialized");
        Self {
            system: System::new_all(),
        }
    }

    pub fn local_stats(&mut self) -> LocalStats {
        self.system.refresh_all();
        LocalStats {
            total_memory_mb: self.system.total_memory() / 1024 / 1024,
            used_memory_mb: self.system.used_memory() / 1024 / 1024,
            cpu_count: self.system.cpus().len() as u32,
            cpu_usage: self.system.global_cpu_usage(),
        }
    }

    /// Test SSH connection to a server (TCP connectivity check for MVP)
    pub async fn test_ssh_connection(
        host: &str,
        port: u16,
        _username: &str,
        _key_pem: Option<&str>,
    ) -> Result<bool> {
        // For MVP, just test TCP connectivity with 10s timeout
        // Full SSH handshake with russh can come later
        let addr = format!("{}:{}", host, port);

        match tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::TcpStream::connect(&addr)
        ).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // timeout
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct LocalStats {
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub cpu_count: u32,
    pub cpu_usage: f32,
}
