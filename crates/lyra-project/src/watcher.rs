use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct FileEvent {
    pub path: PathBuf,
    pub timestamp: f64,
}

impl FileEvent {
    #[must_use]
    pub fn new(path: PathBuf, timestamp: f64) -> Self {
        Self { path, timestamp }
    }
}

#[derive(Clone, Debug)]
pub struct ChangeAccumulator {
    root: PathBuf,
    debounce_interval: f64,
    pending: BTreeMap<PathBuf, f64>,
    latest_timestamp: Option<f64>,
}

impl ChangeAccumulator {
    #[must_use]
    pub fn new(root: PathBuf, debounce_interval: f64) -> Self {
        Self {
            root,
            debounce_interval,
            pending: BTreeMap::new(),
            latest_timestamp: None,
        }
    }

    pub fn ingest(&mut self, events: impl IntoIterator<Item = FileEvent>) {
        for event in events {
            if is_ignored(&self.root, &event.path) {
                continue;
            }
            self.latest_timestamp = Some(
                self.latest_timestamp
                    .map_or(event.timestamp, |latest| latest.max(event.timestamp)),
            );
            self.pending.insert(event.path, event.timestamp);
        }
    }

    #[must_use]
    pub fn poll(&mut self, now: f64) -> Vec<PathBuf> {
        let Some(latest) = self.latest_timestamp else {
            return Vec::new();
        };
        if now - latest < self.debounce_interval {
            return Vec::new();
        }
        self.latest_timestamp = None;
        std::mem::take(&mut self.pending).into_keys().collect()
    }
}

fn is_ignored(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let components: Vec<_> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    components
        .iter()
        .any(|value| matches!(*value, ".build" | ".git" | "target"))
        || components
            .windows(2)
            .any(|window| window == ["Registry", "Site"])
}
