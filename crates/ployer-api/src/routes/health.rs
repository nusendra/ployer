use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use crate::app_state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new().route("/", get(health_check))
}

async fn health_check(State(state): State<SharedState>) -> Json<Value> {
    let docker_ok = match &state.docker {
        Some(docker) => docker.ping().await.unwrap_or(false),
        None => false,
    };

    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .is_ok();

    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "version": env!("CARGO_PKG_VERSION"),
        "services": {
            "database": db_ok,
            "docker": docker_ok,
        }
    }))
}
