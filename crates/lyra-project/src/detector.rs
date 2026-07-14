use std::fs;
use std::path::{Path, PathBuf};

use crate::ProjectError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectMode {
    RepoBound,
    Standalone,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDescriptor {
    pub mode: ProjectMode,
    pub root: PathBuf,
    pub effects_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectDetector;

impl ProjectDetector {
    /// Finds the nearest standalone Pack or repository containing `lyric-effects`.
    ///
    /// # Errors
    ///
    /// Returns an error when the start path is inaccessible or no project marker is found.
    pub fn detect(&self, start: &Path) -> Result<ProjectDescriptor, ProjectError> {
        let canonical_start =
            fs::canonicalize(start).map_err(|error| ProjectError::io(start, error))?;
        let mut candidate = if canonical_start.is_file() {
            canonical_start
                .parent()
                .map(Path::to_owned)
                .ok_or_else(|| ProjectError::UnrecognizedProject(canonical_start.clone()))?
        } else {
            canonical_start.clone()
        };

        loop {
            let effects_root = candidate.join("lyric-effects");
            if effects_root.is_dir() {
                return Ok(ProjectDescriptor {
                    mode: ProjectMode::RepoBound,
                    root: candidate,
                    effects_root,
                });
            }
            if candidate.join("lyra-pack.json").is_file() {
                return Ok(ProjectDescriptor {
                    mode: ProjectMode::Standalone,
                    root: candidate.clone(),
                    effects_root: candidate,
                });
            }
            if !candidate.pop() {
                break;
            }
        }

        Err(ProjectError::UnrecognizedProject(canonical_start))
    }
}
