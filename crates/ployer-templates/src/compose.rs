//! Minimal docker-compose parser. Only supports the subset needed by service
//! templates: image, environment, volumes, networks, ports, restart.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::TemplateError;

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeSpec {
    #[serde(default)]
    pub services: BTreeMap<String, ServiceSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceSpec {
    pub image: String,
    #[serde(default)]
    pub environment: Option<Environment>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub restart: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Environment {
    Map(BTreeMap<String, serde_yaml::Value>),
    List(Vec<String>),
}

impl Environment {
    /// Render to `KEY=value` pairs for docker.
    pub fn to_pairs(&self) -> Vec<String> {
        match self {
            Environment::Map(m) => m
                .iter()
                .map(|(k, v)| format!("{}={}", k, value_to_string(v)))
                .collect(),
            Environment::List(l) => l.clone(),
        }
    }
}

fn value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        _ => serde_yaml::to_string(v).unwrap_or_default().trim().to_string(),
    }
}

pub fn parse(yaml: &str) -> Result<ComposeSpec, TemplateError> {
    Ok(serde_yaml::from_str(yaml)?)
}

/// Split `host:container` or `host:container:mode` into (host, container).
/// Returns None if the format isn't recognized.
pub fn split_volume(spec: &str) -> Option<(String, String)> {
    let mut parts = spec.splitn(3, ':');
    let host = parts.next()?.to_string();
    let container = parts.next()?.to_string();
    Some((host, container))
}

/// Split `host:container` or `container` port spec.
pub fn split_port(spec: &str) -> Option<(String, String)> {
    if let Some((host, container)) = spec.split_once(':') {
        Some((host.to_string(), container.to_string()))
    } else {
        Some((spec.to_string(), spec.to_string()))
    }
}
