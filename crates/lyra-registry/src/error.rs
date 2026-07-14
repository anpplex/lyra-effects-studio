#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported Registry schema version {0}")]
    UnsupportedSchema(u64),
    #[error("duplicate Pack version: {0}")]
    DuplicatePack(String),
    #[error("invalid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid public key length: expected 32, got {0}")]
    PublicKeyLength(usize),
    #[error("invalid signature length: expected 64, got {0}")]
    SignatureLength(usize),
    #[error("invalid public key: {0}")]
    PublicKey(String),
    #[error("Pack contract error: {0}")]
    Pack(#[from] lyra_pack::PackError),
}
