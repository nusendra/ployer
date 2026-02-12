use sysinfo::System;
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
}

#[derive(Debug, serde::Serialize)]
pub struct LocalStats {
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub cpu_count: u32,
    pub cpu_usage: f32,
}
