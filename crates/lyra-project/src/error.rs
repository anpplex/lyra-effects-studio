use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported schema version {0}")]
    UnsupportedSchema(u64),
    #[error("unsafe CSS variable or value")]
    UnsafeCssPatch,
    #[error("CSS does not contain a :root block")]
    MissingRootBlock,
    #[error("no Lyra project found from {0}")]
    UnrecognizedProject(PathBuf),
}

impl ProjectError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
