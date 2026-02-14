use ployer_core::config::AppConfig;
use ployer_core::models::WsEvent;
use ployer_docker::DockerClient;
use ployer_proxy::CaddyClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppState {
    pub db: SqlitePool,
    pub docker: Option<Arc<DockerClient>>,
    pub caddy: CaddyClient,
    pub config: AppConfig,
    pub ws_broadcast: broadcast::Sender<WsEvent>,
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
        Arc::new(Self {
            db,
            docker: docker.map(Arc::new),
            caddy,
            config,
            ws_broadcast,
        })
    }
}
