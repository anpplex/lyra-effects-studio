#![doc = "Theme Pack contracts and deterministic archive support."]

mod archive;
pub mod canonical_json;
mod error;
mod manifest;
mod semantic_version;
mod validator;

pub use archive::{ArchiveResult, PackArchiver};
pub use error::PackError;
pub use manifest::{Author, Compatibility, Entry, License, PackKind, PackManifest};
pub use semantic_version::{SemanticVersion, VersionRange};
pub use validator::{Diagnostic, PackValidator};

use sha2::{Digest, Sha256};
use std::fmt::Write as _;

/// Any JSON value accepted by the public contracts.
pub type CanonicalJson = serde_json::Value;

/// Current public Theme Pack contract version.
pub const FORMAT_VERSION: u64 = 1;

/// Returns the lowercase SHA-256 digest for `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
