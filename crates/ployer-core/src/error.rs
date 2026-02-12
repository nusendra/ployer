use thiserror::Error;

#[derive(Error, Debug)]
pub enum PloyerError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("Proxy error: {0}")]
    Proxy(String),
}

pub type Result<T> = std::result::Result<T, PloyerError>;
