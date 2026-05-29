use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::sync::RwLock;

use crate::error::TemplateError;
use crate::schema::{IndexEntry, RegistryIndex, Template};

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub cache_ttl: Duration,
}

pub struct Registry {
    config: RegistryConfig,
    http: reqwest::Client,
    index_cache: RwLock<Option<(RegistryIndex, SystemTime)>>,
}

impl Registry {
    pub fn new(config: RegistryConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("ployer-templates/0.1")
            .build()
            .expect("reqwest client");
        Self {
            config,
            http,
            index_cache: RwLock::new(None),
        }
    }

    /// Return the catalog. Uses in-memory cache (TTL), falls back to disk cache
    /// when the network is unreachable.
    pub async fn index(&self) -> Result<RegistryIndex, TemplateError> {
        {
            let guard = self.index_cache.read().await;
            if let Some((idx, fetched_at)) = guard.as_ref() {
                if fetched_at.elapsed().unwrap_or(Duration::MAX) < self.config.cache_ttl {
                    return Ok(idx.clone());
                }
            }
        }

        let url = format!("{}/index.json", self.config.registry_url.trim_end_matches('/'));
        match self.fetch_index(&url).await {
            Ok(idx) => {
                self.write_disk_index(&idx).await.ok();
                let mut guard = self.index_cache.write().await;
                *guard = Some((idx.clone(), SystemTime::now()));
                Ok(idx)
            }
            Err(net_err) => match self.read_disk_index().await {
                Ok(idx) => {
                    tracing::warn!("template registry unreachable, serving stale index: {net_err}");
                    Ok(idx)
                }
                Err(_) => Err(net_err),
            },
        }
    }

    /// Fetch a single template by slug. Verifies sha256 against the index.
    pub async fn get(&self, slug: &str) -> Result<Template, TemplateError> {
        let index = self.index().await?;
        let entry = index
            .templates
            .iter()
            .find(|e| e.slug == slug)
            .ok_or_else(|| TemplateError::NotFound(slug.to_string()))?
            .clone();

        if let Ok(cached) = self.read_disk_template(&entry).await {
            return Ok(cached);
        }

        let body = self.http.get(&entry.url).send().await?.error_for_status()?.text().await?;

        let actual = sha256_hex(body.as_bytes());
        if actual != entry.sha256 {
            return Err(TemplateError::HashMismatch {
                slug: slug.to_string(),
                expected: entry.sha256.clone(),
                actual,
            });
        }

        let template: Template = serde_yaml::from_str(&body)?;
        self.write_disk_template(&entry, &body).await.ok();
        Ok(template)
    }

    async fn fetch_index(&self, url: &str) -> Result<RegistryIndex, TemplateError> {
        let body = self.http.get(url).send().await?.error_for_status()?.text().await?;
        let index: RegistryIndex = serde_json::from_str(&body)?;
        Ok(index)
    }

    fn index_path(&self) -> PathBuf {
        self.config.cache_dir.join("index.json")
    }

    fn template_path(&self, entry: &IndexEntry) -> PathBuf {
        self.config.cache_dir.join(format!("{}-{}.yaml", entry.slug, &entry.sha256[..16]))
    }

    async fn ensure_cache_dir(&self) -> Result<(), TemplateError> {
        if !Path::new(&self.config.cache_dir).exists() {
            fs::create_dir_all(&self.config.cache_dir).await?;
        }
        Ok(())
    }

    async fn write_disk_index(&self, index: &RegistryIndex) -> Result<(), TemplateError> {
        self.ensure_cache_dir().await?;
        let body = serde_json::to_vec_pretty(index)?;
        fs::write(self.index_path(), body).await?;
        Ok(())
    }

    async fn read_disk_index(&self) -> Result<RegistryIndex, TemplateError> {
        let body = fs::read(self.index_path()).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    async fn write_disk_template(&self, entry: &IndexEntry, body: &str) -> Result<(), TemplateError> {
        self.ensure_cache_dir().await?;
        fs::write(self.template_path(entry), body).await?;
        Ok(())
    }

    async fn read_disk_template(&self, entry: &IndexEntry) -> Result<Template, TemplateError> {
        let body = fs::read(self.template_path(entry)).await?;
        let actual = sha256_hex(&body);
        if actual != entry.sha256 {
            return Err(TemplateError::HashMismatch {
                slug: entry.slug.clone(),
                expected: entry.sha256.clone(),
                actual,
            });
        }
        Ok(serde_yaml::from_slice(&body)?)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
