use ployer_core::config::AppConfig;
use ployer_core::models::WsEvent;
use ployer_docker::DockerClient;
use ployer_proxy::CaddyClient;
use ployer_templates::{Registry, RegistryConfig};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

pub struct AppState {
    pub db: SqlitePool,
    pub docker: Option<Arc<DockerClient>>,
    pub caddy: CaddyClient,
    pub config: AppConfig,
    pub ws_broadcast: broadcast::Sender<WsEvent>,
    pub templates: Arc<Registry>,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(
        db: SqlitePool,
        docker: Option<DockerClient>,
        caddy: CaddyClient,
        config: AppConfig,
    ) -> SharedState {
        let (ws_broadcast, _) = broadcast::channel(256);
        let templates = Arc::new(Registry::new(RegistryConfig {
            registry_url: config.templates.registry_url.clone(),
            cache_dir: PathBuf::from(&config.templates.cache_dir),
            cache_ttl: Duration::from_secs(config.templates.index_ttl_seconds),
        }));
        Arc::new(Self {
            db,
            docker: docker.map(Arc::new),
            caddy,
            config,
            ws_broadcast,
            templates,
        })
    }
}
