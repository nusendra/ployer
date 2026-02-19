use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde_json::json;
use std::{num::NonZeroU32, sync::Arc};

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>;

/// Create a rate limiter that allows `requests_per_second` burst.
pub fn new_rate_limiter(requests_per_minute: u32) -> SharedRateLimiter {
    let quota = Quota::per_minute(
        NonZeroU32::new(requests_per_minute).expect("rate limit must be > 0"),
    );
    Arc::new(RateLimiter::direct(quota))
}

/// Axum middleware that applies a shared rate limiter to every request.
pub async fn rate_limit_middleware(
    limiter: axum::extract::Extension<SharedRateLimiter>,
    req: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(req).await,
        Err(_) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "Too many requests. Please slow down." })),
        )
            .into_response(),
    }
}
