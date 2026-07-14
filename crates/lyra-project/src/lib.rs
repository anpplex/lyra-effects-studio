#![doc = "Project discovery and editable contracts."]

mod css;
mod detector;
mod diagnostic;
mod error;
mod parameter;
mod profile;
mod scenario;
mod watcher;

pub use css::patch_css_variable;
pub use detector::{ProjectDescriptor, ProjectDetector, ProjectMode};
pub use diagnostic::Diagnostic;
pub use error::ProjectError;
pub use parameter::{
    ParameterBinding, ParameterControl, ParameterDefinition, ParameterGroup, ParameterSchema,
};
pub use profile::{Canvas, DeviceProfile, Insets};
pub use scenario::{LyricLine, PreviewScenario, ScenarioEvent, Track};
pub use watcher::{ChangeAccumulator, FileEvent};

/// Current public project contract version.
pub const FORMAT_VERSION: u64 = 1;
