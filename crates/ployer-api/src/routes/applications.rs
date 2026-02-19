use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;
use crate::middleware::validation;
use ployer_core::crypto;
use ployer_core::models::{Application, BuildStrategy};
use ployer_db::repositories::{ApplicationRepository, DeployKeyRepository, EnvVarRepository};
use ployer_git::GitService;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_applications).post(create_application))
        .route("/:id", get(get_application).put(update_application).delete(delete_application))
        .route("/:id/envs", get(list_env_vars).post(add_env_var))
        .route("/:id/envs/:key", put(update_env_var).delete(delete_env_var))
        .route("/:id/deploy-key", get(get_deploy_key).post(generate_deploy_key))
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct CreateApplicationRequest {
    name: String,
    server_id: String,
    git_url: Option<String>,
    #[serde(default = "default_branch")]
    git_branch: String,
    #[serde(default)]
    build_strategy: BuildStrategy,
    dockerfile_path: Option<String>,
    port: Option<u16>,
    #[serde(default)]
    auto_deploy: bool,
    env_vars: Option<HashMap<String, String>>,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Serialize)]
struct ApplicationResponse {
    application: Application,
}

#[derive(Debug, Serialize)]
struct ListApplicationsResponse {
    applications: Vec<Application>,
}

#[derive(Debug, Deserialize)]
struct UpdateApplicationRequest {
    name: Option<String>,
    git_url: Option<String>,
    git_branch: Option<String>,
    build_strategy: Option<BuildStrategy>,
    dockerfile_path: Option<String>,
    port: Option<u16>,
    auto_deploy: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EnvVarRequest {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct EnvVarResponse {
    key: String,
    value: String, // Decrypted value
}

#[derive(Debug, Serialize)]
struct ListEnvVarsResponse {
    env_vars: Vec<EnvVarResponse>,
}

#[derive(Debug, Serialize)]
struct DeployKeyResponse {
    public_key: String,
    created_at: String,
}

// ===== Handlers =====

async fn list_applications(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<ListApplicationsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ApplicationRepository::new(state.db.clone());
    let applications = repo
        .list()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListApplicationsResponse { applications }))
}

async fn create_application(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<ApplicationResponse>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    validation::required(&req.name, "Name", 100)?;
    validation::required(&req.server_id, "Server ID", 36)?;
    if let Some(ref url) = req.git_url {
        validation::git_url(url)?;
    }
    if let Some(p) = req.port {
        validation::port(p)?;
    }

    let repo = ApplicationRepository::new(state.db.clone());

    // Create application
    let app = repo
        .create(
            &req.name,
            &req.server_id,
            req.git_url.as_deref(),
            &req.git_branch,
            req.build_strategy,
            req.dockerfile_path.as_deref(),
            req.port,
            req.auto_deploy,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Add environment variables if provided
    if let Some(env_vars) = req.env_vars {
        let env_repo = EnvVarRepository::new(state.db.clone());
        let secret_key = state.config.get_secret_key();

        for (key, value) in env_vars {
            let encrypted = crypto::encrypt(&value, &secret_key)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Encryption failed: {}", e)))?;

            env_repo
                .create(&app.id, &key, &encrypted)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    // Note: Deploy key generation is now on-demand via POST /applications/:id/deploy-key
    // This avoids blocking application creation with expensive RSA 4096 key generation

    Ok((StatusCode::CREATED, Json(ApplicationResponse { application: app })))
}

async fn get_application(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApplicationResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ApplicationRepository::new(state.db.clone());
    let app = repo
        .find_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    Ok(Json(ApplicationResponse { application: app }))
}

async fn update_application(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateApplicationRequest>,
) -> Result<Json<ApplicationResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    if let Some(ref name) = req.name {
        validation::required(name, "Name", 100)?;
    }
    if let Some(ref url) = req.git_url {
        validation::git_url(url)?;
    }
    if let Some(p) = req.port {
        validation::port(p)?;
    }

    let repo = ApplicationRepository::new(state.db.clone());

    // Get existing application
    let existing = repo
        .find_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Application not found".to_string()))?;

    // Use new values or keep existing
    let name = req.name.as_deref().unwrap_or(&existing.name);
    let git_url = req.git_url.as_deref().or(existing.git_url.as_deref());
    let git_branch = req.git_branch.as_deref().unwrap_or(&existing.git_branch);
    let build_strategy = req.build_strategy.unwrap_or(existing.build_strategy);
    let dockerfile_path = req.dockerfile_path.as_deref().or(existing.dockerfile_path.as_deref());
    let port = req.port.or(existing.port);
    let auto_deploy = req.auto_deploy.unwrap_or(existing.auto_deploy);

    let app = repo
        .update(&id, name, git_url, git_branch, build_strategy, dockerfile_path, port, auto_deploy)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ApplicationResponse { application: app }))
}

async fn delete_application(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = ApplicationRepository::new(state.db.clone());
    repo.delete(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ===== Environment Variables =====

async fn list_env_vars(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<Json<ListEnvVarsResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = EnvVarRepository::new(state.db.clone());
    let env_vars = repo
        .list_by_application(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Decrypt values
    let secret_key = state.config.get_secret_key();
    let mut decrypted = Vec::new();

    for var in env_vars {
        let value = crypto::decrypt(&var.value_encrypted, &secret_key)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Decryption failed: {}", e)))?;

        decrypted.push(EnvVarResponse {
            key: var.key,
            value,
        });
    }

    Ok(Json(ListEnvVarsResponse { env_vars: decrypted }))
}

async fn add_env_var(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<EnvVarRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    validation::env_key(&req.key)?;

    let secret_key = state.config.get_secret_key();
    let encrypted = crypto::encrypt(&req.value, &secret_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Encryption failed: {}", e)))?;

    let repo = EnvVarRepository::new(state.db.clone());
    repo.create(&app_id, &req.key, &encrypted)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn update_env_var(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, key)): Path<(String, String)>,
    Json(req): Json<EnvVarRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let secret_key = state.config.get_secret_key();
    let encrypted = crypto::encrypt(&req.value, &secret_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Encryption failed: {}", e)))?;

    let repo = EnvVarRepository::new(state.db.clone());
    repo.update(&app_id, &key, &encrypted)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_env_var(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((app_id, key)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = EnvVarRepository::new(state.db.clone());
    repo.delete(&app_id, &key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ===== Deploy Key =====

async fn get_deploy_key(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<Json<DeployKeyResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let repo = DeployKeyRepository::new(state.db.clone());
    let key = repo
        .find_by_application(&app_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Deploy key not found".to_string()))?;

    Ok(Json(DeployKeyResponse {
        public_key: key.public_key,
        created_at: key.created_at.to_rfc3339(),
    }))
}

async fn generate_deploy_key(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Result<(StatusCode, Json<DeployKeyResponse>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Delete existing key if present
    let key_repo = DeployKeyRepository::new(state.db.clone());
    let _ = key_repo.delete(&app_id).await; // Ignore error if no key exists

    // Generate new key pair
    let (public_key, private_key) = GitService::generate_deploy_key()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Key generation failed: {}", e)))?;

    // Encrypt private key
    let secret_key = state.config.get_secret_key();
    let encrypted_private = crypto::encrypt(&private_key, &secret_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Encryption failed: {}", e)))?;

    // Store in database
    let key = key_repo
        .create(&app_id, &public_key, &encrypted_private)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(DeployKeyResponse {
            public_key: key.public_key,
            created_at: key.created_at.to_rfc3339(),
        }),
    ))
}
