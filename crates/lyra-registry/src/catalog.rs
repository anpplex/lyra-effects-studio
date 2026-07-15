use std::collections::{BTreeMap, BTreeSet};

use lyra_pack::{SemanticVersion, canonical_json};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::{FORMAT_VERSION, RegistryError};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPackArtifact {
    pub id: String,
    pub name: String,
    pub family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    pub version: SemanticVersion,
    pub manifest_url: String,
    pub download_url: String,
    pub sha256: String,
    pub signature: String,
    pub size: u64,
    pub minimum_runtime_api: SemanticVersion,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCatalog {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub registry_id: String,
    pub name: String,
    pub generated_at: String,
    pub key_id: String,
    pub packs: Vec<RegistryPackArtifact>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

impl RegistryCatalog {
    /// Decodes a v1 Registry Catalog while preserving unknown fields.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, invalid versions or unsupported schema versions.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, RegistryError> {
        Ok(canonical_json::from_slice(bytes)?)
    }

    /// Encodes the Catalog with the shared canonical JSON rules.
    ///
    /// # Errors
    ///
    /// Returns an error when a contained value cannot be serialized.
    pub fn to_canonical_vec(&self) -> Result<Vec<u8>, RegistryError> {
        Ok(canonical_json::to_vec(self)?)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryBuilder;

impl RegistryBuilder {
    /// Builds a deterministically sorted v1 Catalog.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::DuplicatePack`] for repeated Pack ID/version pairs.
    pub fn build(
        registry_id: &str,
        name: &str,
        generated_at: &str,
        key_id: &str,
        mut packs: Vec<RegistryPackArtifact>,
    ) -> Result<RegistryCatalog, RegistryError> {
        let mut versions = BTreeSet::new();
        for pack in &packs {
            let identity = (pack.id.clone(), pack.version.to_string());
            if !versions.insert(identity.clone()) {
                return Err(RegistryError::DuplicatePack(format!(
                    "{}@{}",
                    identity.0, identity.1
                )));
            }
        }
        packs.sort_by(|left, right| (&left.id, &left.version).cmp(&(&right.id, &right.version)));
        Ok(RegistryCatalog {
            schema_version: FORMAT_VERSION,
            registry_id: registry_id.into(),
            name: name.into(),
            generated_at: generated_at.into(),
            key_id: key_id.into(),
            packs,
            additional: BTreeMap::new(),
        })
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
