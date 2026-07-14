use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use lyra_pack::{PackManifest, sha256_hex};
use lyra_project::{ProjectDetector, ProjectMode};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

const MAX_EDITABLE_SOURCE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditablePack {
    id: String,
    name: String,
    version: String,
    family: String,
    root: PathBuf,
    style_path: PathBuf,
    style_source: String,
    style_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectSnapshot {
    root: PathBuf,
    effects_root: PathBuf,
    mode: String,
    packs: Vec<EditablePack>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveStyleRequest {
    pack_root: PathBuf,
    expected_sha256: String,
    source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SaveStatus {
    Saved,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveStyleResult {
    status: SaveStatus,
    sha256: String,
}

#[tauri::command]
pub(crate) fn open_project(path: &str) -> Result<ProjectSnapshot, String> {
    load_project(Path::new(path))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn save_project_style(request: SaveStyleRequest) -> Result<SaveStyleResult, String> {
    save_style(&request)
}

fn load_project(start: &Path) -> Result<ProjectSnapshot, String> {
    let descriptor = ProjectDetector
        .detect(start)
        .map_err(|error| error.to_string())?;
    let manifest_paths = match descriptor.mode {
        ProjectMode::Standalone => vec![descriptor.root.join("lyra-pack.json")],
        ProjectMode::RepoBound => WalkDir::new(&descriptor.effects_root)
            .follow_links(false)
            .max_depth(8)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && entry.file_name() == "lyra-pack.json")
            .map(walkdir::DirEntry::into_path)
            .collect(),
    };

    let mut packs = manifest_paths
        .into_iter()
        .map(|path| load_editable_pack(&descriptor.effects_root, &path))
        .collect::<Result<Vec<_>, _>>()?;
    packs.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(ProjectSnapshot {
        root: descriptor.root,
        effects_root: descriptor.effects_root,
        mode: match descriptor.mode {
            ProjectMode::RepoBound => "repo-bound",
            ProjectMode::Standalone => "standalone",
        }
        .into(),
        packs,
    })
}

fn load_editable_pack(effects_root: &Path, manifest_path: &Path) -> Result<EditablePack, String> {
    let manifest_bytes = fs::read(manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest = PackManifest::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    let pack_root = manifest_path
        .parent()
        .ok_or_else(|| "Pack manifest has no parent directory".to_owned())?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let effects_root = effects_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !pack_root.starts_with(&effects_root) {
        return Err("Pack root escapes the detected project".into());
    }

    let relative_style = manifest
        .entry
        .style
        .as_deref()
        .ok_or_else(|| format!("Pack {} has no style entry", manifest.id))?;
    let style_path = pack_root
        .join(relative_style)
        .canonicalize()
        .map_err(|error| format!("failed to resolve style entry: {error}"))?;
    if !style_path.starts_with(&pack_root) {
        return Err("Style entry escapes the Pack root".into());
    }
    let style_bytes = fs::read(&style_path)
        .map_err(|error| format!("failed to read {}: {error}", style_path.display()))?;
    if style_bytes.len() > MAX_EDITABLE_SOURCE_BYTES {
        return Err("Style source exceeds the 2 MiB editor limit".into());
    }
    let style_source = String::from_utf8(style_bytes.clone())
        .map_err(|_| "Style source must be valid UTF-8".to_owned())?;

    Ok(EditablePack {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version.to_string(),
        family: manifest.family,
        root: pack_root,
        style_path,
        style_source,
        style_sha256: sha256_hex(&style_bytes),
    })
}

fn save_style(request: &SaveStyleRequest) -> Result<SaveStyleResult, String> {
    if request.source.is_empty()
        || request.source.len() > MAX_EDITABLE_SOURCE_BYTES
        || request.source.contains('\0')
    {
        return Err("Style source is empty, contains NUL, or exceeds the 2 MiB limit".into());
    }
    let snapshot = load_project(&request.pack_root)?;
    let canonical_pack_root = request
        .pack_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let pack = snapshot
        .packs
        .iter()
        .find(|pack| pack.root == canonical_pack_root)
        .ok_or_else(|| "Pack is not part of the detected project".to_owned())?;
    if pack.style_sha256 != request.expected_sha256 {
        return Ok(SaveStyleResult {
            status: SaveStatus::Conflict,
            sha256: pack.style_sha256.clone(),
        });
    }

    let parent = pack
        .style_path
        .parent()
        .ok_or_else(|| "Style path has no parent directory".to_owned())?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|error| format!("save failed: {error}"))?;
    temporary
        .write_all(request.source.as_bytes())
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|error| format!("save failed: {error}"))?;
    temporary
        .persist(&pack.style_path)
        .map_err(|error| format!("save failed: {}", error.error))?;

    Ok(SaveStyleResult {
        status: SaveStatus::Saved,
        sha256: sha256_hex(request.source.as_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{SaveStatus, SaveStyleRequest, load_project, save_style};

    #[test]
    fn opens_a_standalone_pack_and_returns_editable_style_source() {
        let project = standalone_project();

        let snapshot = load_project(project.path()).expect("load project");

        assert_eq!(snapshot.mode, "standalone");
        assert_eq!(snapshot.packs.len(), 1);
        assert_eq!(snapshot.packs[0].id, "io.lyra.test.theme");
        assert_eq!(
            snapshot.packs[0].style_source,
            ":root { --lyra-size: 42px; }\n"
        );
        assert_eq!(snapshot.packs[0].style_sha256.len(), 64);
    }

    #[test]
    fn saves_only_when_the_expected_source_hash_matches() {
        let project = standalone_project();
        let snapshot = load_project(project.path()).expect("load project");
        let pack = &snapshot.packs[0];
        let request = SaveStyleRequest {
            pack_root: pack.root.clone(),
            expected_sha256: pack.style_sha256.clone(),
            source: ":root { --lyra-size: 48px; }\n".into(),
        };

        let result = save_style(&request).expect("save style");

        assert_eq!(result.status, SaveStatus::Saved);
        assert_eq!(
            fs::read_to_string(project.path().join("theme/lyra.css")).expect("read style"),
            request.source
        );
    }

    #[test]
    fn reports_external_conflicts_without_overwriting() {
        let project = standalone_project();
        let snapshot = load_project(project.path()).expect("load project");
        let pack = &snapshot.packs[0];
        fs::write(project.path().join("theme/lyra.css"), "external change\n")
            .expect("external write");

        let result = save_style(&SaveStyleRequest {
            pack_root: pack.root.clone(),
            expected_sha256: pack.style_sha256.clone(),
            source: "editor change\n".into(),
        })
        .expect("conflict response");

        assert_eq!(result.status, SaveStatus::Conflict);
        assert_eq!(
            fs::read_to_string(project.path().join("theme/lyra.css")).expect("read style"),
            "external change\n"
        );
    }

    fn standalone_project() -> TempDir {
        let root = TempDir::new().expect("project root");
        fs::create_dir(root.path().join("theme")).expect("theme directory");
        fs::write(
            root.path().join("lyra-pack.json"),
            r#"{
              "schemaVersion": 1,
              "id": "io.lyra.test.theme",
              "name": "Test Theme",
              "version": "1.0.0",
              "kind": "theme",
              "family": "test",
              "author": { "name": "Lyra" },
              "license": { "spdx": "MIT" },
              "compatibility": {
                "packSchema": ">=1 <2",
                "runtimeApi": ">=1.0.0 <2.0.0",
                "bridgeApi": ">=1.0.0 <2.0.0"
              },
              "entry": { "style": "theme/lyra.css" },
              "capabilities": ["styles"]
            }"#,
        )
        .expect("manifest");
        fs::write(
            root.path().join("theme/lyra.css"),
            ":root { --lyra-size: 42px; }\n",
        )
        .expect("style");
        root
    }
}
