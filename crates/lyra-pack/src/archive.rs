use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::{PackError, PackValidator, sha256_hex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveResult {
    pub path: PathBuf,
    pub sha256: String,
    pub byte_count: u64,
    pub entries: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PackArchiver {
    validator: PackValidator,
}

impl PackArchiver {
    /// Validates and builds a deterministic ZIP archive.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails or source/destination I/O cannot complete.
    pub fn build(&self, source: &Path, destination: &Path) -> Result<ArchiveResult, PackError> {
        let diagnostics = self.validator.validate(source)?;
        if !diagnostics.is_empty() {
            return Err(PackError::Validation(
                diagnostics.into_iter().map(|item| item.code).collect(),
            ));
        }

        let mut entries = collect_files(source)?;
        entries.sort();
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| PackError::io(parent, error))?;
        }

        let output =
            File::create(destination).map_err(|error| PackError::io(destination, error))?;
        let mut writer = ZipWriter::new(output);
        let timestamp = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
            .map_err(|error| PackError::VersionRange(error.to_string()))?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(timestamp)
            .unix_permissions(0o644);

        for relative in &entries {
            writer.start_file(relative, options)?;
            let path = source.join(relative);
            let mut input = File::open(&path).map_err(|error| PackError::io(&path, error))?;
            std::io::copy(&mut input, &mut writer)
                .map_err(|error| PackError::io(destination, error))?;
        }
        writer.finish()?;

        let mut archive =
            File::open(destination).map_err(|error| PackError::io(destination, error))?;
        let mut bytes = Vec::new();
        archive
            .read_to_end(&mut bytes)
            .map_err(|error| PackError::io(destination, error))?;

        Ok(ArchiveResult {
            path: destination.to_owned(),
            sha256: sha256_hex(&bytes),
            byte_count: bytes.len() as u64,
            entries,
        })
    }
}

fn collect_files(root: &Path) -> Result<Vec<String>, PackError> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| PackError::VersionRange(error.to_string()))?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(files)
}
