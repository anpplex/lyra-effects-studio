use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::{FORMAT_VERSION, PackError, SemanticVersion, VersionRange, canonical_json};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackKind {
    Theme,
    WebEffect,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Author {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct License {
    pub spdx: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub pack_schema: VersionRange,
    pub runtime_api: VersionRange,
    pub bridge_api: VersionRange,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(rename = "themeId", skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub id: String,
    pub name: String,
    pub version: SemanticVersion,
    pub kind: PackKind,
    pub family: String,
    pub author: Author,
    pub license: License,
    pub compatibility: Compatibility,
    pub entry: Entry,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

impl PackManifest {
    /// Decodes a v1 Pack manifest while preserving unknown fields.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, invalid SemVer/ranges or unsupported schema versions.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, PackError> {
        let manifest: Self = canonical_json::from_slice(bytes)?;
        manifest.validate_contract()?;
        Ok(manifest)
    }

    /// Encodes this manifest as canonical JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if a contained value cannot be serialized.
    pub fn to_canonical_vec(&self) -> Result<Vec<u8>, PackError> {
        canonical_json::to_vec(self)
    }

    fn validate_contract(&self) -> Result<(), PackError> {
        if !is_valid_pack_id(&self.id) {
            return Err(PackError::Contract(format!("invalid id: {}", self.id)));
        }
        if self.name.is_empty() || self.family.is_empty() || self.author.name.is_empty() {
            return Err(PackError::Contract(
                "name, family and author.name must not be empty".into(),
            ));
        }
        let unique: BTreeSet<_> = self.capabilities.iter().collect();
        if unique.len() != self.capabilities.len() {
            return Err(PackError::Contract(
                "capabilities must contain unique values".into(),
            ));
        }
        if self.family == "better-lyrics" {
            let Some(theme_id) = self.entry.theme_id.as_deref() else {
                return Err(PackError::Contract(
                    "Better Lyrics manifests require entry.themeId".into(),
                ));
            };
            if !is_valid_theme_id(theme_id) {
                return Err(PackError::Contract(format!(
                    "invalid Better Lyrics themeId: {theme_id}"
                )));
            }
        }
        Ok(())
    }
}

fn is_valid_pack_id(id: &str) -> bool {
    let mut segments = id.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    if first.is_empty()
        || !first
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return false;
    }
    let remainder: Vec<_> = segments.collect();
    !remainder.is_empty()
        && remainder.iter().all(|segment| {
            segment
                .as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn is_valid_theme_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes[0] != b'-'
        && bytes[bytes.len() - 1] != b'-'
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn deserialize_schema_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let version = u64::deserialize(deserializer)?;
    if version == FORMAT_VERSION {
        Ok(version)
    } else {
        Err(D::Error::custom(PackError::UnsupportedSchema(version)))
    }
}
