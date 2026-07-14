use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::{FORMAT_VERSION, RegistryError};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncludedTheme {
    pub theme_id: String,
    pub pack_id: String,
    pub version: String,
    pub source_repository: String,
    pub source_revision: String,
    #[serde(rename = "sourceURL")]
    pub source_url: String,
    #[serde(rename = "sourceCSSPath")]
    pub source_css_path: String,
    #[serde(rename = "sourceCSSSHA256")]
    pub source_css_sha256: String,
    #[serde(rename = "licenseSPDX")]
    pub license_spdx: String,
    #[serde(rename = "licenseEvidenceURL")]
    pub license_evidence_url: String,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedTheme {
    pub theme_id: String,
    pub pack_id: String,
    pub version: String,
    pub source_repository: String,
    pub source_revision: String,
    pub reason_code: String,
    pub reason: String,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseAudit {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub generated_at: String,
    pub source_catalog_path: String,
    pub source_revision: String,
    pub included: Vec<IncludedTheme>,
    pub excluded: Vec<ExcludedTheme>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryDiagnostic {
    pub code: String,
    pub pack_id: String,
    pub message: String,
}

impl LicenseAudit {
    /// Decodes a v1 license audit.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON or unsupported schema versions.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, RegistryError> {
        Ok(serde_json::from_slice(bytes)?)
    }

    #[must_use]
    pub fn validate_sources(&self, registry_root: &Path) -> Vec<RegistryDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut identities = BTreeSet::new();
        for item in &self.included {
            if !identities.insert(&item.pack_id) {
                diagnostics.push(diagnostic(
                    "license.packDuplicate",
                    &item.pack_id,
                    "Pack appears more than once in the audit",
                ));
            }
            let css = registry_root
                .join("Packs")
                .join(&item.pack_id)
                .join("theme/lyra.css");
            match fs::read(&css) {
                Ok(bytes) if lyra_pack::sha256_hex(&bytes) == item.source_css_sha256 => {}
                Ok(_) => diagnostics.push(diagnostic(
                    "license.sourceHashMismatch",
                    &item.pack_id,
                    "imported CSS does not match the audited SHA-256",
                )),
                Err(_) => diagnostics.push(diagnostic(
                    "license.sourceMissing",
                    &item.pack_id,
                    "imported CSS is missing",
                )),
            }
        }
        for item in &self.excluded {
            if !identities.insert(&item.pack_id) {
                diagnostics.push(diagnostic(
                    "license.packDuplicate",
                    &item.pack_id,
                    "Pack appears more than once in the audit",
                ));
            }
            if item.reason_code.is_empty() {
                diagnostics.push(diagnostic(
                    "license.reasonMissing",
                    &item.pack_id,
                    "excluded Pack needs a reason code",
                ));
            }
        }
        diagnostics
    }
}

fn diagnostic(code: &str, pack_id: &str, message: &str) -> RegistryDiagnostic {
    RegistryDiagnostic {
        code: code.into(),
        pack_id: pack_id.into(),
        message: message.into(),
    }
}

fn deserialize_schema_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let version = u64::deserialize(deserializer)?;
    if version == FORMAT_VERSION {
        Ok(version)
    } else {
        Err(D::Error::custom(RegistryError::UnsupportedSchema(version)))
    }
}
