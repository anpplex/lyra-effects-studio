#![doc = "Signed Theme Registry contracts."]

mod catalog;
mod crypto;
mod error;
mod license_audit;

pub use catalog::{RegistryBuilder, RegistryCatalog, RegistryPackArtifact};
pub use crypto::{RegistrySigner, RegistryVerifier};
pub use error::RegistryError;
pub use license_audit::{ExcludedTheme, IncludedTheme, LicenseAudit, RegistryDiagnostic};

/// Current public Registry contract version.
pub const FORMAT_VERSION: u64 = 1;
