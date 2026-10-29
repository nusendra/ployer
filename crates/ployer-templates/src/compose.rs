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

/// Remove any `ports` mapping from every service in the compose YAML.
pub fn remove_ports_from_all_services(yaml: &str) -> Result<String, TemplateError> {
    let mut root: serde_yaml::Value = serde_yaml::from_str(yaml)?;
    if let Some(services) = root.get_mut("services").and_then(|s| s.as_mapping_mut()) {
        for (_, svc) in services.iter_mut() {
            if let Some(svc_map) = svc.as_mapping_mut() {
                svc_map.remove(serde_yaml::Value::String("ports".to_string()));
            }
        }
    }
    Ok(serde_yaml::to_string(&root)?)
}

/// Read the current host:container port bindings declared in the compose,
/// flattened across all services. Useful for showing current state in the UI.
/// The host interface, if the spec names one, is dropped — see
/// [`parse_port_spec`] when you need it.
pub fn current_port_bindings(yaml: &str) -> Result<Vec<(u16, u16)>, TemplateError> {
    let spec: ComposeSpec = serde_yaml::from_str(yaml)?;
    let mut out = Vec::new();
    for (_, svc) in spec.services {
        for port_str in svc.ports {
            if let Some(mapping) = parse_port_spec(&port_str) {
                if let (Ok(h), Ok(c)) = (
                    mapping.host_port.parse::<u16>(),
                    mapping.container_port.parse::<u16>(),
                ) {
                    out.push((c, h));
                }
            }
        }
    }
    Ok(out)
}

/// Inject a `ports` mapping into every service in the compose YAML. Used by
/// the install endpoint when the user opts to expose a template's port to
/// the host. `bindings` maps `container_port -> host_port`, and `host_ip` is
/// the interface to publish on — always written out explicitly so the stored
/// compose records which interface was chosen rather than leaving it to
/// whatever the deployer's default happens to be.
pub fn add_ports_to_all_services(
    yaml: &str,
    bindings: &[(u16, u16)],
    host_ip: &str,
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
            serde_yaml::Value::String(format_port_spec(host_ip, *host, *container))
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

/// A parsed docker-compose short-syntax port mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMapping {
    /// Host interface the port is published on. `None` when the spec does not
    /// name one (`"15432:5432"`), which leaves the choice to the deployer.
    pub host_ip: Option<String>,
    pub host_port: String,
    pub container_port: String,
}

/// Parse one compose short-syntax `ports:` entry.
///
/// Handles `container`, `host:container` and `host_ip:host:container`,
/// including a bracketed IPv6 host address (`[::1]:15432:5432`). Returns
/// `None` for anything else, including empty components.
///
/// Note that a bare `"5432"` is read as host 5432 -> container 5432. Compose
/// itself would give that an ephemeral host port; Ployer has always read it
/// the other way and templates never emit the bare form, so this keeps the
/// existing behaviour rather than silently moving anyone's port.
pub fn parse_port_spec(spec: &str) -> Option<PortMapping> {
    let spec = spec.trim();

    // Bracketed IPv6 host address: [::1]:15432:5432
    if let Some(rest) = spec.strip_prefix('[') {
        let (ip, rest) = rest.split_once(']')?;
        let (host_port, container_port) = rest.strip_prefix(':')?.split_once(':')?;
        return port_mapping(Some(ip), host_port, container_port);
    }

    let parts: Vec<&str> = spec.split(':').collect();
    match parts.as_slice() {
        [port] => port_mapping(None, port, port),
        [host_port, container_port] => port_mapping(None, host_port, container_port),
        [ip, host_port, container_port] => port_mapping(Some(ip), host_port, container_port),
        _ => None,
    }
}

fn port_mapping(ip: Option<&str>, host_port: &str, container_port: &str) -> Option<PortMapping> {
    if host_port.is_empty() || container_port.is_empty() {
        return None;
    }
    Some(PortMapping {
        host_ip: ip.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        host_port: host_port.to_string(),
        container_port: container_port.to_string(),
    })
}

/// Split a port spec into `(host_port, container_port)`, discarding any host
/// interface. Prefer [`parse_port_spec`], which keeps it.
pub fn split_port(spec: &str) -> Option<(String, String)> {
    parse_port_spec(spec).map(|m| (m.host_port, m.container_port))
}

/// Render a compose `ports:` entry. An empty `host_ip` omits the interface;
/// an IPv6 literal is bracketed so it round-trips through [`parse_port_spec`].
fn format_port_spec(host_ip: &str, host_port: u16, container_port: u16) -> String {
    let ip = host_ip.trim();
    if ip.is_empty() {
        format!("{}:{}", host_port, container_port)
    } else if ip.contains(':') {
        format!("[{}]:{}:{}", ip, host_port, container_port)
    } else {
        format!("{}:{}:{}", ip, host_port, container_port)
    }
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;
