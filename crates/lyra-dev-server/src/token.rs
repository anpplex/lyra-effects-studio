use crate::ServerDiagnostic;

pub(crate) struct BridgeToken([u8; 32]);

impl Clone for BridgeToken {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl BridgeToken {
    /// Generates a token from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns `device.bridge.tokenGenerationFailed` when random bytes are unavailable.
    pub(crate) fn generate() -> Result<Self, ServerDiagnostic> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| {
            ServerDiagnostic::new("device.bridge.tokenGenerationFailed", error.to_string())
        })?;
        Ok(Self(bytes))
    }

    #[cfg(test)]
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn authorization_value(&self) -> String {
        format!("Bearer {}", hex(&self.0))
    }

    pub(crate) fn matches_authorization(&self, value: Option<&axum::http::HeaderValue>) -> bool {
        value
            .and_then(|header| header.to_str().ok())
            .is_some_and(|header| header == self.authorization_value())
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}
