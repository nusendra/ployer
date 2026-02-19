use axum::{
    extract::State,
    http::StatusCode,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use ployer_core::models::User;

use crate::app_state::SharedState;
use crate::auth::{validate_token, AuthService};
use crate::middleware::validation;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    user: User,
    token: String,
}

async fn register(
    State(state): State<SharedState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let auth_service = AuthService::new(state.db.clone());

    validation::email(&req.email)?;
    validation::password(&req.password)?;
    validation::required(&req.name, "Name", 100)?;

    // Register user
    let user = auth_service
        .register(&req.email, &req.password, &req.name)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Generate token
    let token = crate::auth::jwt::generate_token(
        &user.id,
        &user.email,
        user.role.as_str(),
        &state.config.auth.jwt_secret,
        state.config.auth.token_expiry_hours,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegisterResponse { user, token }))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user: User,
    token: String,
}

async fn login(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let auth_service = AuthService::new(state.db.clone());

    // Login
    let (user, token) = auth_service
        .login(
            &req.email,
            &req.password,
            &state.config.auth.jwt_secret,
            state.config.auth.token_expiry_hours,
        )
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    Ok(Json(LoginResponse { user, token }))
}

#[derive(Debug, Serialize)]
struct MeResponse {
    user: User,
}

async fn me(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    // Extract and validate token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid Authorization header format".to_string()))?;

    let claims = validate_token(token, &state.config.auth.jwt_secret)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()))?;

    // Get user
    let auth_service = AuthService::new(state.db.clone());
    let user = auth_service
        .get_user(&claims.sub)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(MeResponse { user }))
}
