mod app_state;
mod auth;
mod middleware;
mod routes;
mod services;
mod websocket;

use anyhow::Result;
use axum::{middleware as axum_middleware, Extension, Router};
use clap::{Parser, Subcommand};
use ployer_core::config::AppConfig;
use ployer_docker::DockerClient;
use ployer_proxy::CaddyClient;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

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
    /// Reset a user's password
    ResetPassword {
        /// User email address
        #[arg(long)]
        email: String,
        /// New password (min 8 chars)
        #[arg(long)]
        password: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Determine log format from env var (LOG_FORMAT=json for structured logging)
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_default();

    if log_format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    } else {
        fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

    let cli = Cli::parse();

    // Load config (use defaults for now)
    let config = AppConfig::default();

    match cli.command.unwrap_or(Commands::Start) {
        Commands::Start => start_server(config).await,
        Commands::Migrate => run_migrations(config).await,
        Commands::ResetPassword { email, password } => {
            reset_password(config, &email, &password).await
        }
    }
}

async fn run_migrations(config: AppConfig) -> Result<()> {
    let pool = ployer_db::create_pool(&config.database.url).await?;
    ployer_db::run_migrations(&pool).await?;
    info!("Migrations complete");
    Ok(())
}

async fn reset_password(config: AppConfig, email: &str, password: &str) -> Result<()> {
    if password.len() < 8 {
        anyhow::bail!("Password must be at least 8 characters");
    }

    let pool = ployer_db::create_pool(&config.database.url).await?;
    let repo = ployer_db::repositories::UserRepository::new(pool.clone());

    let user = repo
        .find_by_email(email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User '{}' not found", email))?;

    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    repo.update_password(&user.id, &hash).await?;
    info!("Password reset for user '{}'", email);
    Ok(())
}

async fn register_local_server(pool: &sqlx::SqlitePool) -> Result<()> {
    use ployer_core::models::ServerStatus;
    use ployer_db::repositories::ServerRepository;

    let repo = ServerRepository::new(pool.clone());

    // Check if local server already exists
    if repo.find_local().await?.is_some() {
        return Ok(());
    }

    // Get hostname
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "local".to_string());

    // Create local server
    let server = repo
        .create(&hostname, "localhost", 22, "root", None, true)
        .await?;

    // Set initial status to online
    repo.update_status(&server.id, ServerStatus::Online, chrono::Utc::now())
        .await?;

    info!("Local server registered: {}", hostname);
    Ok(())
}

/// Build a CorsLayer from the configured allowed_origins string.
fn build_cors(allowed_origins: &str) -> CorsLayer {
    if allowed_origins == "*" {
        return CorsLayer::permissive();
    }

    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
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
    let cors = build_cors(&config.server.allowed_origins);

    // Rate limiter: 300 req/min globally
    let rate_limiter = middleware::rate_limit::new_rate_limiter(300);

    // Build shared state
    let state = app_state::AppState::new(pool.clone(), docker, caddy, config);

    // Start health monitors
    services::health_monitor::spawn_health_monitor(pool.clone(), state.ws_broadcast.clone());
    services::app_health_monitor::spawn_app_health_monitor(
        pool.clone(),
        state.docker.clone(),
        state.ws_broadcast.clone(),
    );

    // Start stats aggregator
    services::stats_aggregator::spawn_stats_aggregator(pool, state.docker.clone());

    // Build router
    let app = Router::new()
        .nest("/api/v1", routes::api_router())
        .layer(axum_middleware::from_fn(
            middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(Extension(rate_limiter))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    info!("Ployer server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Graceful shutdown on SIGTERM or Ctrl-C
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl-C, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}
