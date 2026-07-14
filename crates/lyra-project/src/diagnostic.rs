#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub path: Option<String>,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(code: &str, path: Option<String>, message: &str) -> Self {
        Self {
            code: code.into(),
            path,
            message: message.into(),
        }
    }
}
