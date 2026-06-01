use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::app_state::SharedState;
use crate::auth::extract_user_id;

const GITHUB_REPO: &str = "nusendra/ployer";
const INSTALL_URL: &str = "https://ployer.nusendra.com/install.sh";
const LATEST_CACHE_TTL: Duration = Duration::from_secs(600);

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/version", get(get_version))
        .route("/update", post(trigger_update))
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    current: String,
    latest: Option<String>,
    update_available: bool,
}

struct LatestCache {
    version: String,
    fetched_at: Instant,
}

fn latest_cache() -> &'static Mutex<Option<LatestCache>> {
    static CACHE: OnceLock<Mutex<Option<LatestCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

async fn fetch_latest_version() -> Option<String> {
    {
        let guard = latest_cache().lock().await;
        if let Some(c) = guard.as_ref() {
            if c.fetched_at.elapsed() < LATEST_CACHE_TTL {
                return Some(c.version.clone());
            }
        }
    }

    let url = format!("https://api.github.com/repos/{}/releases", GITHUB_REPO);
    let client = reqwest::Client::builder()
        .user_agent("ployer")
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let releases: Vec<Value> = client.get(&url).send().await.ok()?.json().await.ok()?;

    let mut tags: Vec<String> = releases
        .iter()
        .filter_map(|r| r.get("tag_name").and_then(|v| v.as_str()).map(String::from))
        .collect();
    tags.sort_by(|a, b| version_key(a).cmp(&version_key(b)));
    let latest = tags.into_iter().last()?;
    let stripped = latest.trim_start_matches('v').to_string();

    let mut guard = latest_cache().lock().await;
    *guard = Some(LatestCache {
        version: stripped.clone(),
        fetched_at: Instant::now(),
    });
    Some(stripped)
}

// Coarse semver-ish sort key — splits on '.' and '-', numeric parts numeric-sorted,
// pre-release tags (alpha < beta < rc < stable) ordered below stable.
fn version_key(v: &str) -> Vec<(u32, String)> {
    let v = v.trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let mut parts: Vec<(u32, String)> = core
        .split('.')
        .map(|p| (p.parse::<u32>().unwrap_or(0), String::new()))
        .collect();
    // Stable releases sort above any prerelease
    parts.push((if pre.is_some() { 0 } else { 1 }, String::new()));
    if let Some(pre) = pre {
        for seg in pre.split('.') {
            if let Ok(n) = seg.parse::<u32>() {
                parts.push((n, String::new()));
            } else {
                parts.push((0, seg.to_string()));
            }
        }
    }
    parts
}

async fn get_version(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<VersionResponse>, (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    let current = env!("PLOYER_VERSION").to_string();
    let latest = fetch_latest_version().await;
    let update_available = match &latest {
        Some(l) => version_key(l) > version_key(&current),
        None => false,
    };

    Ok(Json(VersionResponse {
        current,
        latest,
        update_available,
    }))
}

async fn trigger_update(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    extract_user_id(&headers, &state.config.auth.jwt_secret)?;

    // Run the installer as a transient systemd unit so it survives `systemctl stop ployer`
    // (which would otherwise kill any child process in our cgroup).
    let cmd = format!("curl -fsSL {} | bash", INSTALL_URL);
    let status = tokio::process::Command::new("systemd-run")
        .args([
            "--unit=ployer-updater",
            "--collect",
            "--description=Ployer Self-Update",
            "bash",
            "-c",
            &cmd,
        ])
        .status()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to spawn updater: {}", e),
            )
        })?;

    if !status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("systemd-run exited with {}", status),
        ));
    }

    tracing::info!("Update triggered via systemd-run (ployer-updater)");
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "started",
            "message": "Update started in background. Service will restart automatically."
        })),
    ))
}
