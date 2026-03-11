use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("path resolution failed: {0}")]
    PathResolution(String),
    #[error("DB open/create failed: {0}")]
    DbOpen(#[from] rusqlite::Error),
    #[error("schema mismatch: expected version {expected}, found {found}")]
    SchemaMismatch { expected: u32, found: u32 },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("deserialization error: {0}")]
    Deserialization(String),
    #[error("invariant failure: {0}")]
    InvariantFailure(String),
    #[error("save not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
