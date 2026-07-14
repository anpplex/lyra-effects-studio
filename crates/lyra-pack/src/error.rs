use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("I/O failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid semantic version: {0}")]
    SemanticVersion(#[from] semver::Error),
    #[error("unsupported Pack schema version {0}")]
    UnsupportedSchema(u64),
    #[error("invalid version range: {0}")]
    VersionRange(String),
    #[error("invalid Pack contract: {0}")]
    Contract(String),
    #[error("failed to walk Pack: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("ZIP operation failed: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Pack validation failed: {0:?}")]
    Validation(Vec<String>),
}

impl PackError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
