use serde::Serialize;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[error("{code}: {message}")]
pub struct ServerDiagnostic {
    pub code: String,
    pub message: String,
}

impl ServerDiagnostic {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
