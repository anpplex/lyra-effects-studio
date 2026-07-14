use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};

use crate::{RegistryCatalog, RegistryError};

#[derive(Clone)]
pub struct RegistrySigner {
    private_key: SigningKey,
}

impl RegistrySigner {
    #[must_use]
    pub fn from_private_key_bytes(bytes: [u8; 32]) -> Self {
        Self {
            private_key: SigningKey::from_bytes(&bytes),
        }
    }

    #[must_use]
    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.private_key.verifying_key().to_bytes())
    }

    /// Signs the canonical Catalog bytes and returns standard padded base64.
    ///
    /// # Errors
    ///
    /// Returns an error if the Catalog cannot be serialized.
    pub fn sign_catalog(&self, catalog: &RegistryCatalog) -> Result<String, RegistryError> {
        let bytes = catalog.to_canonical_vec()?;
        Ok(STANDARD.encode(self.private_key.sign(&bytes).to_bytes()))
    }

    #[must_use]
    pub fn sign_checksum(&self, checksum: &str) -> String {
        STANDARD.encode(self.private_key.sign(checksum.as_bytes()).to_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct RegistryVerifier {
    public_key: VerifyingKey,
}

impl RegistryVerifier {
    /// Builds a verifier from a raw Ed25519 public key encoded as base64.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid base64, length or curve encoding.
    pub fn from_public_key_base64(source: &str) -> Result<Self, RegistryError> {
        let bytes = STANDARD.decode(source)?;
        let length = bytes.len();
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| RegistryError::PublicKeyLength(length))?;
        let public_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|error| RegistryError::PublicKey(error.to_string()))?;
        Ok(Self { public_key })
    }

    /// Verifies a detached signature over canonical Catalog bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization, base64 decoding or signature length validation fails.
    pub fn verify_catalog(
        &self,
        catalog: &RegistryCatalog,
        signature_base64: &str,
    ) -> Result<bool, RegistryError> {
        let bytes = catalog.to_canonical_vec()?;
        let signature = decode_signature(signature_base64)?;
        Ok(self.public_key.verify(&bytes, &signature).is_ok())
    }

    #[must_use]
    pub fn verify_pack(&self, data: &[u8], expected_sha256: &str, signature_base64: &str) -> bool {
        if lyra_pack::sha256_hex(data) != expected_sha256 {
            return false;
        }
        decode_signature(signature_base64).is_ok_and(|signature| {
            self.public_key
                .verify(expected_sha256.as_bytes(), &signature)
                .is_ok()
        })
    }
}

fn decode_signature(source: &str) -> Result<Signature, RegistryError> {
    let bytes = STANDARD.decode(source)?;
    let length = bytes.len();
    let bytes: [u8; 64] = bytes
        .try_into()
        .map_err(|_| RegistryError::SignatureLength(length))?;
    Ok(Signature::from_bytes(&bytes))
}
