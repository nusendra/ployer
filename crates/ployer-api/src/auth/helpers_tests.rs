use super::*;
use axum::http::HeaderValue;
use crate::auth::jwt::generate_token;

const SECRET: &str = "test-jwt-secret";

fn make_headers(auth_value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("Authorization", HeaderValue::from_str(auth_value).unwrap());
    headers
}

#[test]
fn valid_bearer_token_returns_user_id() {
    let token = generate_token("user-abc", "u@example.com", "user", SECRET, 1).unwrap();
    let headers = make_headers(&format!("Bearer {}", token));
    let user_id = extract_user_id(&headers, SECRET).unwrap();
    assert_eq!(user_id, "user-abc");
}

#[test]
fn missing_authorization_header_returns_unauthorized() {
    let headers = HeaderMap::new();
    let result = extract_user_id(&headers, SECRET);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
}

#[test]
fn missing_bearer_prefix_returns_unauthorized() {
    let token = generate_token("user-abc", "u@example.com", "user", SECRET, 1).unwrap();
    let headers = make_headers(&token); // no "Bearer " prefix
    let result = extract_user_id(&headers, SECRET);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
}

#[test]
fn invalid_token_returns_unauthorized() {
    let headers = make_headers("Bearer invalid.token.here");
    let result = extract_user_id(&headers, SECRET);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
}

#[test]
fn token_signed_with_wrong_secret_returns_unauthorized() {
    let token = generate_token("user-abc", "u@example.com", "user", "other-secret", 1).unwrap();
    let headers = make_headers(&format!("Bearer {}", token));
    let result = extract_user_id(&headers, SECRET);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
}
