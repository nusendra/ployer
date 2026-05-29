use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub schema_version: u32,
    pub generated_at: String,
    pub templates: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: serde_json::Value,
    pub sha256: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: serde_json::Value,
    #[serde(default)]
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
    pub compose: String,
    #[serde(default)]
    pub post_install: Option<PostInstall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: InputKind,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub generate: Option<Generate>,
    #[serde(default)]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    String,
    Password,
    Number,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generate {
    pub kind: GenerateKind,
    #[serde(default)]
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerateKind {
    Password,
    Hex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub container: u16,
    #[serde(default)]
    pub expose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub name: String,
    pub mount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInstall {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub outputs: Vec<OutputItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputItem {
    pub label: String,
    pub value: String,
}
