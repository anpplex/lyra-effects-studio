use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use lyra_pack::{PackManifest, sha256_hex};
use lyra_project::{ParameterSchema, PreviewScenario, ProjectDetector, ProjectMode};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

const MAX_EDITABLE_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PARAMETER_SCHEMA_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize)]
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
    parameters: Option<ParameterSchema>,
    scenarios: Vec<PreviewScenario>,
    documents: Vec<EditableDocument>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EditableDocumentKind {
    Css,
    Html,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditableDocument {
    id: String,
    label: String,
    kind: EditableDocumentKind,
    path: PathBuf,
    relative_path: PathBuf,
    source: String,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveDocumentRequest {
    pack_root: PathBuf,
    document_path: PathBuf,
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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn save_project_document(
    request: SaveDocumentRequest,
) -> Result<SaveStyleResult, String> {
    save_document(&request)
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
    let parameters = manifest
        .parameters
        .as_deref()
        .map(|relative| load_parameter_schema(&pack_root, relative))
        .transpose()?;
    let mut documents = vec![load_editable_document(
        &pack_root,
        "style".into(),
        "Styles".into(),
        relative_style,
        EditableDocumentKind::Css,
    )?];
    if let Some(relative) = manifest.entry.html.as_deref() {
        documents.push(load_editable_document(
            &pack_root,
            "html".into(),
            "HTML".into(),
            relative,
            EditableDocumentKind::Html,
        )?);
    }
    if let Some(relative) = manifest.parameters.as_deref() {
        documents.push(load_editable_document(
            &pack_root,
            "parameters".into(),
            "Parameters".into(),
            relative,
            EditableDocumentKind::Json,
        )?);
    }
    let (scenarios, scenario_documents) = load_scenario_documents(&pack_root, &manifest.scenarios)?;
    documents.extend(scenario_documents);

    Ok(EditablePack {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version.to_string(),
        family: manifest.family,
        root: pack_root,
        style_path,
        style_source,
        style_sha256: sha256_hex(&style_bytes),
        parameters,
        scenarios,
        documents,
    })
}

fn load_scenario_documents(
    pack_root: &Path,
    relative_paths: &[String],
) -> Result<(Vec<PreviewScenario>, Vec<EditableDocument>), String> {
    let mut scenarios = Vec::with_capacity(relative_paths.len());
    let mut documents = Vec::with_capacity(relative_paths.len());
    for (index, relative) in relative_paths.iter().enumerate() {
        let document = load_editable_document(
            pack_root,
            format!("scenario-{index}"),
            format!("Scenario {}", index + 1),
            relative,
            EditableDocumentKind::Json,
        )?;
        let scenario = PreviewScenario::from_slice(document.source.as_bytes())
            .map_err(|error| format!("failed to parse scenario: {error}"))?;
        let diagnostics = scenario.validate();
        if !diagnostics.is_empty() {
            return Err(format!(
                "Scenario validation failed: {}",
                diagnostics
                    .iter()
                    .map(|item| item.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        scenarios.push(scenario);
        documents.push(document);
    }
    Ok((scenarios, documents))
}

fn load_editable_document(
    pack_root: &Path,
    id: String,
    label: String,
    relative: &str,
    kind: EditableDocumentKind,
) -> Result<EditableDocument, String> {
    let path = pack_root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("failed to resolve editable document: {error}"))?;
    if !path.starts_with(pack_root) {
        return Err("Editable document escapes the Pack root".into());
    }
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if bytes.len() > MAX_EDITABLE_SOURCE_BYTES {
        return Err("Editable document exceeds the 2 MiB editor limit".into());
    }
    let source = String::from_utf8(bytes.clone())
        .map_err(|_| "Editable document must be valid UTF-8".to_owned())?;
    Ok(EditableDocument {
        id,
        label,
        kind,
        path,
        relative_path: PathBuf::from(relative),
        source,
        sha256: sha256_hex(&bytes),
    })
}

fn load_parameter_schema(pack_root: &Path, relative: &str) -> Result<ParameterSchema, String> {
    let path = pack_root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("failed to resolve parameter schema: {error}"))?;
    if !path.starts_with(pack_root) {
        return Err("Parameter schema escapes the Pack root".into());
    }
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if bytes.len() > MAX_PARAMETER_SCHEMA_BYTES {
        return Err("Parameter schema exceeds the 512 KiB limit".into());
    }
    let schema = ParameterSchema::from_slice(&bytes).map_err(|error| error.to_string())?;
    let diagnostics = schema.validate();
    if !diagnostics.is_empty() {
        let codes = diagnostics
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Parameter schema validation failed: {codes}"));
    }
    Ok(schema)
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

fn save_document(request: &SaveDocumentRequest) -> Result<SaveStyleResult, String> {
    if request.source.is_empty()
        || request.source.len() > MAX_EDITABLE_SOURCE_BYTES
        || request.source.contains('\0')
    {
        return Err("Document source is empty, contains NUL, or exceeds the 2 MiB limit".into());
    }
    let snapshot = load_project(&request.pack_root)?;
    let canonical_pack_root = request
        .pack_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let canonical_document_path = request
        .document_path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let pack = snapshot
        .packs
        .iter()
        .find(|pack| pack.root == canonical_pack_root)
        .ok_or_else(|| "Pack is not part of the detected project".to_owned())?;
    let document = pack
        .documents
        .iter()
        .find(|document| document.path == canonical_document_path)
        .ok_or_else(|| "Document is not declared by the Pack manifest".to_owned())?;
    if document.sha256 != request.expected_sha256 {
        return Ok(SaveStyleResult {
            status: SaveStatus::Conflict,
            sha256: document.sha256.clone(),
        });
    }
    if document.kind == EditableDocumentKind::Json {
        validate_json_document(document, &request.source)?;
    }

    let parent = document
        .path
        .parent()
        .ok_or_else(|| "Document path has no parent directory".to_owned())?;
    let mut temporary =
        NamedTempFile::new_in(parent).map_err(|error| format!("save failed: {error}"))?;
    temporary
        .write_all(request.source.as_bytes())
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|error| format!("save failed: {error}"))?;
    temporary
        .persist(&document.path)
        .map_err(|error| format!("save failed: {}", error.error))?;

    Ok(SaveStyleResult {
        status: SaveStatus::Saved,
        sha256: sha256_hex(request.source.as_bytes()),
    })
}

fn validate_json_document(document: &EditableDocument, source: &str) -> Result<(), String> {
    if document.id == "parameters" {
        let schema =
            ParameterSchema::from_slice(source.as_bytes()).map_err(|error| error.to_string())?;
        let diagnostics = schema.validate();
        if !diagnostics.is_empty() {
            return Err(format!(
                "Parameter schema validation failed: {}",
                diagnostics
                    .iter()
                    .map(|item| item.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    } else if document.id.starts_with("scenario-") {
        let scenario =
            PreviewScenario::from_slice(source.as_bytes()).map_err(|error| error.to_string())?;
        let diagnostics = scenario.validate();
        if !diagnostics.is_empty() {
            return Err(format!(
                "Scenario validation failed: {}",
                diagnostics
                    .iter()
                    .map(|item| item.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    } else {
        serde_json::from_str::<serde_json::Value>(source)
            .map_err(|error| format!("JSON syntax error: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        SaveDocumentRequest, SaveStatus, SaveStyleRequest, load_project, save_document, save_style,
    };

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
        assert_eq!(snapshot.packs[0].documents.len(), 4);
        assert_eq!(snapshot.packs[0].documents[0].id, "style");
        assert_eq!(snapshot.packs[0].documents[1].id, "html");
        assert_eq!(snapshot.packs[0].documents[2].id, "parameters");
        assert_eq!(snapshot.packs[0].documents[3].id, "scenario-0");
        assert_eq!(snapshot.packs[0].scenarios[0].id, "io.lyra.scenario.test");
        let parameters = snapshot.packs[0]
            .parameters
            .as_ref()
            .expect("parameter schema");
        assert_eq!(parameters.groups[0].id, "appearance");
        assert_eq!(parameters.groups[0].parameters[0].id, "font-size");
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
    fn saves_only_manifest_declared_documents() {
        let project = standalone_project();
        let snapshot = load_project(project.path()).expect("load project");
        let pack = &snapshot.packs[0];
        let document = pack
            .documents
            .iter()
            .find(|document| document.id == "parameters")
            .expect("parameters document");
        let source = document.source.replace("42", "48");

        let result = save_document(&SaveDocumentRequest {
            pack_root: pack.root.clone(),
            document_path: document.path.clone(),
            expected_sha256: document.sha256.clone(),
            source: source.clone(),
        })
        .expect("save document");

        assert_eq!(result.status, SaveStatus::Saved);
        assert_eq!(
            fs::read_to_string(project.path().join("parameters.json")).expect("read parameters"),
            source
        );

        let undeclared = project.path().join("notes.txt");
        fs::write(&undeclared, "private notes\n").expect("undeclared fixture");
        let error = save_document(&SaveDocumentRequest {
            pack_root: pack.root.clone(),
            document_path: undeclared.clone(),
            expected_sha256: "ignored".into(),
            source: "overwritten\n".into(),
        })
        .expect_err("undeclared documents must be rejected");
        assert!(error.contains("not declared"));
        assert_eq!(
            fs::read_to_string(undeclared).expect("read undeclared fixture"),
            "private notes\n"
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
              "entry": { "style": "theme/lyra.css", "html": "theme/index.html" },
              "parameters": "parameters.json",
              "scenarios": ["scenarios/default.json"],
              "capabilities": ["styles"]
            }"#,
        )
        .expect("manifest");
        fs::write(
            root.path().join("parameters.json"),
            r#"{
              "schemaVersion": 1,
              "groups": [{
                "id": "appearance",
                "label": "Appearance",
                "parameters": [{
                  "id": "font-size",
                  "label": "Font size",
                  "control": "length",
                  "binding": { "cssVariable": "--lyra-size" },
                  "defaultValue": 42,
                  "unit": "px",
                  "minimum": 24,
                  "maximum": 72,
                  "step": 1
                }]
              }]
            }"#,
        )
        .expect("parameters");
        fs::write(
            root.path().join("theme/index.html"),
            "<main id=\"blyrics-wrapper\"></main>\n",
        )
        .expect("html");
        fs::create_dir_all(root.path().join("scenarios")).expect("scenario directory");
        fs::write(
            root.path().join("scenarios/default.json"),
            r#"{
              "schemaVersion": 1,
              "id": "io.lyra.scenario.test",
              "track": { "title": "Midnight Galaxy", "artist": "Future Echoes" },
              "lyrics": [{ "startMilliseconds": 0, "endMilliseconds": 4000, "text": "Across the stars" }],
              "events": [],
              "expectedDiagnostics": []
            }"#,
        )
        .expect("scenario");
        fs::write(
            root.path().join("theme/lyra.css"),
            ":root { --lyra-size: 42px; }\n",
        )
        .expect("style");
        root
    }
}
