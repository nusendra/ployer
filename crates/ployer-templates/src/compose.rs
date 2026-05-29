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

/// Inject a `ports` mapping into every service in the compose YAML. Used by
/// the install endpoint when the user opts to expose a template's port to
/// the host. `bindings` maps `container_port -> host_port`.
pub fn add_ports_to_all_services(
    yaml: &str,
    bindings: &[(u16, u16)],
) -> Result<String, TemplateError> {
    if bindings.is_empty() {
        return Ok(yaml.to_string());
    }

    let mut root: serde_yaml::Value = serde_yaml::from_str(yaml)?;
    let services = root
        .get_mut("services")
        .and_then(|s| s.as_mapping_mut())
        .ok_or_else(|| TemplateError::InvalidInput {
            key: "compose".to_string(),
            reason: "missing services map".to_string(),
        })?;

    let port_seq: Vec<serde_yaml::Value> = bindings
        .iter()
        .map(|(container, host)| {
            serde_yaml::Value::String(format!("{}:{}", host, container))
        })
        .collect();

    for (_, svc) in services.iter_mut() {
        if let Some(svc_map) = svc.as_mapping_mut() {
            svc_map.insert(
                serde_yaml::Value::String("ports".to_string()),
                serde_yaml::Value::Sequence(port_seq.clone()),
            );
        }
    }

    Ok(serde_yaml::to_string(&root)?)
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
