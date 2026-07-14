use std::fmt::{Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::PackError;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SemanticVersion(semver::Version);

impl SemanticVersion {
    /// Parses a strict Semantic Version 2.0 string.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::SemanticVersion`] when `source` is not strict `SemVer`.
    pub fn parse(source: &str) -> Result<Self, PackError> {
        Ok(Self(semver::Version::parse(source)?))
    }
}

impl Display for SemanticVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl Serialize for SemanticVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SemanticVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        Self::parse(&source).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operator {
    GreaterThanOrEqual,
    GreaterThan,
    LessThanOrEqual,
    LessThan,
    Equal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Comparator {
    operator: Operator,
    version: SemanticVersion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionRange {
    source: String,
    comparators: Vec<Comparator>,
}

impl VersionRange {
    /// Parses a whitespace-separated conjunction such as `>=1 <2`.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::VersionRange`] for unsupported operators or malformed versions.
    pub fn parse(source: &str) -> Result<Self, PackError> {
        let comparators = source
            .split_whitespace()
            .map(parse_comparator)
            .collect::<Result<Vec<_>, _>>()?;
        if comparators.is_empty() {
            return Err(PackError::VersionRange(source.into()));
        }
        Ok(Self {
            source: source.into(),
            comparators,
        })
    }

    #[must_use]
    pub fn contains(&self, version: &SemanticVersion) -> bool {
        self.comparators
            .iter()
            .all(|comparator| match comparator.operator {
                Operator::GreaterThanOrEqual => version >= &comparator.version,
                Operator::GreaterThan => version > &comparator.version,
                Operator::LessThanOrEqual => version <= &comparator.version,
                Operator::LessThan => version < &comparator.version,
                Operator::Equal => version == &comparator.version,
            })
    }
}

impl Display for VersionRange {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.source)
    }
}

impl Serialize for VersionRange {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.source)
    }
}

impl<'de> Deserialize<'de> for VersionRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        Self::parse(&source).map_err(D::Error::custom)
    }
}

fn parse_comparator(source: &str) -> Result<Comparator, PackError> {
    let (operator, version) = [
        (">=", Operator::GreaterThanOrEqual),
        ("<=", Operator::LessThanOrEqual),
        (">", Operator::GreaterThan),
        ("<", Operator::LessThan),
        ("=", Operator::Equal),
    ]
    .into_iter()
    .find_map(|(prefix, operator)| source.strip_prefix(prefix).map(|rest| (operator, rest)))
    .ok_or_else(|| PackError::VersionRange(source.into()))?;

    let component_count = version.split('.').count();
    let normalized = match component_count {
        1 => format!("{version}.0.0"),
        2 => format!("{version}.0"),
        3 => version.to_owned(),
        _ => return Err(PackError::VersionRange(source.into())),
    };

    Ok(Comparator {
        operator,
        version: SemanticVersion::parse(&normalized)
            .map_err(|_| PackError::VersionRange(source.into()))?,
    })
}
