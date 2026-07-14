use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

use crate::{Diagnostic, FORMAT_VERSION, ProjectError};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Track {
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_milliseconds: u64,
    pub end_milliseconds: u64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transliteration: Option<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioEvent {
    pub at_milliseconds: u64,
    pub kind: String,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewScenario {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub id: String,
    pub track: Track,
    pub lyrics: Vec<LyricLine>,
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub expected_diagnostics: Vec<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

impl PreviewScenario {
    /// Decodes a v1 preview scenario.
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
        if self.track.artwork.as_deref().is_some_and(is_remote_url) {
            diagnostics.push(Diagnostic::new(
                "scenario.remoteAssetForbidden",
                Some("track.artwork".into()),
                "preview scenarios must use local assets",
            ));
        }
        for (index, lyric) in self.lyrics.iter().enumerate() {
            if lyric.end_milliseconds <= lyric.start_milliseconds {
                diagnostics.push(Diagnostic::new(
                    "scenario.lyricTimingInvalid",
                    Some(format!("lyrics[{index}]")),
                    "lyric end must be after start",
                ));
            }
        }
        diagnostics
    }
}

fn is_remote_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn deserialize_schema_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let version = u64::deserialize(deserializer)?;
    if version == FORMAT_VERSION {
        Ok(version)
    } else {
        Err(D::Error::custom(ProjectError::UnsupportedSchema(version)))
    }
}
