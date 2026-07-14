use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::{Value, json};

use crate::{Diagnostic, FORMAT_VERSION, ProjectError};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Canvas {
    pub physical_width: u32,
    pub physical_height: u32,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

const fn default_scale() -> f64 {
    1.0
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Insets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u64,
    pub id: String,
    pub name: String,
    pub canvas: Canvas,
    pub safe_area: Insets,
    #[serde(default)]
    pub masks: Vec<Value>,
    #[serde(default)]
    pub adb: Value,
    pub capabilities: Vec<String>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

impl DeviceProfile {
    #[must_use]
    pub fn built_in_avatr_star_ring() -> Self {
        Self {
            schema_version: FORMAT_VERSION,
            id: "com.avatr.cluster.4032x284".into(),
            name: "Avatr Star Ring 4032×284".into(),
            canvas: Canvas {
                physical_width: 4032,
                physical_height: 284,
                scale: 1.0,
                additional: BTreeMap::new(),
            },
            safe_area: Insets {
                top: 16,
                right: 64,
                bottom: 16,
                left: 64,
            },
            masks: Vec::new(),
            adb: json!({"displayId": null}),
            capabilities: vec!["devBridge".into(), "remoteScreen".into()],
            additional: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.canvas.physical_width == 0 || self.canvas.physical_height == 0 {
            diagnostics.push(Diagnostic::new(
                "profile.canvasInvalid",
                None,
                "canvas dimensions must be positive",
            ));
        }
        if self.safe_area.left + self.safe_area.right >= self.canvas.physical_width
            || self.safe_area.top + self.safe_area.bottom >= self.canvas.physical_height
        {
            diagnostics.push(Diagnostic::new(
                "profile.safeAreaInvalid",
                None,
                "safe area must fit within the canvas",
            ));
        }
        diagnostics
    }
}

fn deserialize_schema_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let version = u64::deserialize(deserializer)?;
    if version == FORMAT_VERSION {
        Ok(version)
    } else {
        Err(D::Error::custom(ProjectError::UnsupportedSchema(version)))
    }
}
