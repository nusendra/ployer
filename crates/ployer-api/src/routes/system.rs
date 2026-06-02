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

// install.sh uses BINARY_ARCH=x86_64 for amd64 and BINARY_ARCH=arm64 for
// aarch64, and asset names are `ployer-{tag}-ployer-linux-{BINARY_ARCH}.tar.gz`.
// We need to match the same naming so we only consider releases whose tarball
// for this host's arch has actually been uploaded — otherwise the dashboard
// would advertise an "update" that the installer can't download yet (e.g. while
// the GitHub Actions release job is still running), which leaves the service
// broken until someone re-runs the installer by hand.
fn current_binary_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}

fn release_has_binary_for_this_host(release: &Value, tag: &str) -> bool {
    let needle = format!(
        "ployer-{}-ployer-linux-{}.tar.gz",
        tag,
        current_binary_arch()
    );
    release
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter().any(|asset| {
                asset.get("name").and_then(|n| n.as_str()) == Some(needle.as_str())
            })
        })
        .unwrap_or(false)
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

    // Pair each release with its tag, drop drafts, sort high → low, then pick
    // the highest version whose tarball for this host's arch is uploaded.
    let mut candidates: Vec<(String, &Value)> = releases
        .iter()
        .filter(|r| !r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false))
        .filter_map(|r| {
            r.get("tag_name")
                .and_then(|v| v.as_str())
                .map(|t| (t.to_string(), r))
        })
        .collect();
    candidates.sort_by(|a, b| version_key(&b.0).cmp(&version_key(&a.0)));

    let latest_tag = candidates
        .into_iter()
        .find(|(tag, r)| release_has_binary_for_this_host(r, tag))
        .map(|(tag, _)| tag)?;
    let stripped = latest_tag.trim_start_matches('v').to_string();

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
