use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::{Diagnostic, FORMAT_VERSION, ProjectError};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParameterControl {
    Color,
    Length,
    Toggle,
    Number,
    Text,
    Select,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterBinding {
    pub css_variable: String,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterDefinition {
    pub id: String,
    pub label: String,
    pub control: ParameterControl,
    pub binding: ParameterBinding,
    pub default_value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ParameterGroup {
    pub id: String,
    pub label: String,
    pub parameters: Vec<ParameterDefinition>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterSchema {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub groups: Vec<ParameterGroup>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

impl ParameterSchema {
    /// Decodes a v1 parameter schema.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON or an unsupported schema version.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ProjectError> {
        Ok(serde_json::from_slice(bytes)?)
    }

    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut ids = BTreeSet::new();
        for group in &self.groups {
            for parameter in &group.parameters {
                if !ids.insert(&parameter.id) {
                    diagnostics.push(Diagnostic::new(
                        "parameter.idDuplicate",
                        Some(parameter.id.clone()),
                        "parameter IDs must be unique",
                    ));
                }
                if !valid_css_variable(&parameter.binding.css_variable) {
                    diagnostics.push(Diagnostic::new(
                        "binding.cssVariableInvalid",
                        Some(parameter.id.clone()),
                        "binding must be a CSS custom property",
                    ));
                }
                if let Some(default) = parameter.default_value.as_f64()
                    && (parameter.minimum.is_some_and(|minimum| default < minimum)
                        || parameter.maximum.is_some_and(|maximum| default > maximum))
                {
                    diagnostics.push(Diagnostic::new(
                        "parameter.defaultOutOfRange",
                        Some(parameter.id.clone()),
                        "numeric default is outside declared bounds",
                    ));
                }
            }
        }
        diagnostics
    }
}

fn valid_css_variable(value: &str) -> bool {
    value.strip_prefix("--").is_some_and(|name| {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

fn deserialize_schema_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let version = u64::deserialize(deserializer)?;
    if version == FORMAT_VERSION {
        Ok(version)
    } else {
        Err(D::Error::custom(ProjectError::UnsupportedSchema(version)))
    }
}
