use serde::{Serialize, de::DeserializeOwned};

use crate::PackError;

/// Serializes a value with recursively sorted object keys, compact separators and one newline.
///
/// # Errors
///
/// Returns [`PackError::Json`] when the value cannot be represented as JSON.
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, PackError> {
    let value = sort_value(serde_json::to_value(value)?);
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sort_value).collect())
        }
        serde_json::Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let values = entries
                .into_iter()
                .map(|(key, value)| (key, sort_value(value)))
                .collect();
            serde_json::Value::Object(values)
        }
        scalar => scalar,
    }
}

/// Deserializes exactly one JSON value, allowing only trailing whitespace.
///
/// # Errors
///
/// Returns [`PackError::Json`] when the input is invalid or has trailing non-whitespace bytes.
pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, PackError> {
    Ok(serde_json::from_slice(bytes)?)
}
