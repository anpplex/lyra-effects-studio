use std::fs;
use std::path::{Component, Path};

use walkdir::WalkDir;

use crate::{PackError, PackKind, PackManifest};

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub path: Option<String>,
    pub message: String,
}

impl Diagnostic {
    fn new(code: &str, path: Option<&Path>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            path: path.map(|value| value.to_string_lossy().into_owned()),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackValidator {
    max_file_bytes: u64,
    max_pack_bytes: u64,
}

impl Default for PackValidator {
    fn default() -> Self {
        Self {
            max_file_bytes: MAX_FILE_BYTES,
            max_pack_bytes: MAX_PACK_BYTES,
        }
    }
}

impl PackValidator {
    /// Creates a validator with explicit byte budgets, primarily for hosts with tighter limits.
    #[must_use]
    pub const fn with_budgets(max_file_bytes: u64, max_pack_bytes: u64) -> Self {
        Self {
            max_file_bytes,
            max_pack_bytes,
        }
    }

    /// Validates the manifest, referenced files and Pack filesystem boundary.
    ///
    /// # Errors
    ///
    /// Returns an error when the Pack cannot be read or its manifest cannot be decoded.
    pub fn validate(&self, root: &Path) -> Result<Vec<Diagnostic>, PackError> {
        let manifest_path = root.join("lyra-pack.json");
        let manifest_bytes =
            fs::read(&manifest_path).map_err(|error| PackError::io(&manifest_path, error))?;
        let manifest = PackManifest::from_slice(&manifest_bytes)?;
        let canonical_root = fs::canonicalize(root).map_err(|error| PackError::io(root, error))?;
        let mut diagnostics = Vec::new();

        if manifest.kind == PackKind::Theme && manifest.entry.script.is_some() {
            diagnostics.push(Diagnostic::new(
                "theme.scriptForbidden",
                manifest.entry.script.as_deref().map(Path::new),
                "Theme Packs cannot declare a script entry",
            ));
        }

        for entry_path in [
            manifest.entry.style.as_deref(),
            manifest.entry.html.as_deref(),
            manifest.entry.script.as_deref(),
            manifest.parameters.as_deref(),
        ]
        .into_iter()
        .flatten()
        .chain(manifest.scenarios.iter().map(String::as_str))
        {
            validate_reference(root, entry_path, &mut diagnostics);
        }

        let mut total_bytes = 0_u64;
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type();

            if file_type.is_symlink() {
                let target_escapes = fs::canonicalize(path)
                    .map_or(true, |target| !target.starts_with(&canonical_root));
                if target_escapes {
                    diagnostics.push(Diagnostic::new(
                        "symlink.escapesRoot",
                        path.strip_prefix(root).ok(),
                        "symbolic link resolves outside the Pack root",
                    ));
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let metadata = entry.metadata()?;
            total_bytes = total_bytes.saturating_add(metadata.len());
            if metadata.len() > self.max_file_bytes {
                diagnostics.push(Diagnostic::new(
                    "file.tooLarge",
                    path.strip_prefix(root).ok(),
                    format!("file exceeds {} bytes", self.max_file_bytes),
                ));
            }
            if manifest.kind == PackKind::Theme
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("js"))
            {
                diagnostics.push(Diagnostic::new(
                    "theme.scriptForbidden",
                    path.strip_prefix(root).ok(),
                    "Theme Packs cannot contain JavaScript",
                ));
            }
            if is_executable(path, &metadata) {
                diagnostics.push(Diagnostic::new(
                    "file.executableForbidden",
                    path.strip_prefix(root).ok(),
                    "Pack files cannot be executable or use executable extensions",
                ));
            }
        }

        if total_bytes > self.max_pack_bytes {
            diagnostics.push(Diagnostic::new(
                "pack.tooLarge",
                None,
                format!("Pack exceeds {} bytes", self.max_pack_bytes),
            ));
        }

        diagnostics
            .sort_by(|left, right| (&left.code, &left.path).cmp(&(&right.code, &right.path)));
        Ok(diagnostics)
    }
}

fn validate_reference(root: &Path, reference: &str, diagnostics: &mut Vec<Diagnostic>) {
    let path = Path::new(reference);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        diagnostics.push(Diagnostic::new(
            "path.traversal",
            Some(path),
            "Pack paths must remain relative to the Pack root",
        ));
    }
    if !root.join(path).is_file() {
        diagnostics.push(Diagnostic::new(
            "entry.missing",
            Some(path),
            "referenced Pack file does not exist",
        ));
    }
}

fn is_executable(path: &Path, metadata: &fs::Metadata) -> bool {
    has_executable_extension(path) || has_executable_mode(metadata)
}

fn has_executable_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "appimage"
                    | "apk"
                    | "bash"
                    | "bat"
                    | "cmd"
                    | "com"
                    | "exe"
                    | "fish"
                    | "jar"
                    | "msi"
                    | "ps1"
                    | "scr"
                    | "sh"
                    | "zsh"
            )
        })
}

#[cfg(unix)]
fn has_executable_mode(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_executable_mode(_metadata: &fs::Metadata) -> bool {
    false
}
