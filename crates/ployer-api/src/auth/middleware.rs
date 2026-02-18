use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use crate::app_state::SharedState;
use super::jwt::validate_token;

/// Middleware to validate JWT tokens from Authorization header
#[allow(dead_code)]
pub async fn auth_middleware(
    State(state): State<SharedState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check Bearer token format
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = validate_token(token, &state.config.auth.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Insert user_id into request extensions for downstream handlers
    req.extensions_mut().insert(claims.sub.clone());

    Ok(next.run(req).await)
}
