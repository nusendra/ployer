use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("template not found: {0}")]
    NotFound(String),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid template yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("invalid index json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("sha256 mismatch for {slug}: expected {expected}, got {actual}")]
    HashMismatch {
        slug: String,
        expected: String,
        actual: String,
    },

    #[error("missing required input: {0}")]
    MissingInput(String),

    #[error("invalid input {key}: {reason}")]
    InvalidInput { key: String, reason: String },
}
