use thiserror::Error;

#[derive(Debug, Error)]
pub enum LintErrorKind {
    #[error("Failed to parse JSON file: {0}")]
    JsonParseError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
