mod app_state;
mod auth;
mod routes;
mod services;

use anyhow::Result;
use axum::Router;
use clap::{Parser, Subcommand};
use ployer_core::config::AppConfig;
use ployer_docker::DockerClient;
use ployer_proxy::CaddyClient;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "ployer", about = "Lightweight self-hosting PaaS")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Ployer server
    Start,
    /// Run database migrations
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config (use defaults for now)
    let config = AppConfig::default();

    match cli.command.unwrap_or(Commands::Start) {
        Commands::Start => start_server(config).await,
        Commands::Migrate => run_migrations(config).await,
    }
}

async fn run_migrations(config: AppConfig) -> Result<()> {
    let pool = ployer_db::create_pool(&config.database.url).await?;
    ployer_db::run_migrations(&pool).await?;
    info!("Migrations complete");
    Ok(())
}

async fn register_local_server(pool: &sqlx::SqlitePool) -> Result<()> {
    use ployer_db::repositories::ServerRepository;
    use ployer_core::models::ServerStatus;

    let repo = ServerRepository::new(pool.clone());

    // Check if local server already exists
    if repo.find_local().await?.is_some() {
        return Ok(());
    }

    // Get hostname
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "local".to_string());

    // Create local server
    let server = repo.create(
        &hostname,
        "localhost",
        22,
        "root",
        None,
        true,
    ).await?;

    // Set initial status to online
    repo.update_status(&server.id, ServerStatus::Online, chrono::Utc::now()).await?;

    info!("Local server registered: {}", hostname);

    Ok(())
}

async fn start_server(config: AppConfig) -> Result<()> {
    // Database
    let pool = ployer_db::create_pool(&config.database.url).await?;
    ployer_db::run_migrations(&pool).await?;

    // Auto-register local server if not exists
    register_local_server(&pool).await?;

    // Docker (optional — don't fail if Docker isn't available)
    let docker = match DockerClient::new(&config.docker.socket_path) {
        Ok(client) => {
            if client.ping().await.unwrap_or(false) {
                info!("Docker connected");
                Some(client)
            } else {
                tracing::warn!("Docker socket found but ping failed");
                Some(client)
            }
        }
        Err(e) => {
            tracing::warn!("Docker not available: {}", e);
            None
        }
    };

    // Caddy client
    let caddy = CaddyClient::new(&config.caddy.admin_url);

    let addr = format!("{}:{}", config.server.host, config.server.port);

    // Build shared state
    let state = app_state::AppState::new(pool.clone(), docker, caddy, config);

    // Start health monitor
    services::health_monitor::spawn_health_monitor(pool, state.ws_broadcast.clone());

    // Build router
    let app = Router::new()
        .nest("/api/v1", routes::api_router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    info!("Ployer server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
