use std::fmt;

use crate::DeviceDiagnostic;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RevisionId(String);

impl RevisionId {
    /// Creates an immutable revision identifier from a lowercase SHA-256 digest.
    ///
    /// # Errors
    ///
    /// Returns `device.revision.invalidId` for malformed digests.
    pub fn from_sha256(source: &str) -> Result<Self, DeviceDiagnostic> {
        if source.len() != 64
            || !source
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DeviceDiagnostic::with_code(
                "device.revision.invalidId",
                "revision ID must be 64 lowercase hexadecimal characters",
            ));
        }
        Ok(Self(source.into()))
    }
}

impl fmt::Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionState {
    Draft,
    LocallyValidated,
    Staged,
    Active,
    Ready,
    Rejected { code: String },
    Superseded,
    Failed { code: String },
    RolledBack { restored: RevisionId },
    Restored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionEvent {
    Validate,
    Stage,
    Reject { code: String },
    Activate { previous_ready: Option<RevisionId> },
    Ready,
    Supersede,
    Fail { code: String },
    Rollback,
    RestoreFormalPack,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionMachine {
    revision_id: RevisionId,
    state: RevisionState,
    previous_ready: Option<RevisionId>,
}

impl RevisionMachine {
    #[must_use]
    pub fn new(revision_id: RevisionId) -> Self {
        Self {
            revision_id,
            state: RevisionState::Draft,
            previous_ready: None,
        }
    }

    #[must_use]
    pub const fn state(&self) -> &RevisionState {
        &self.state
    }

    #[must_use]
    pub const fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }

    /// Applies one legal revision lifecycle event atomically.
    ///
    /// # Errors
    ///
    /// Returns `device.revision.invalidTransition` and preserves state when the event is illegal.
    pub fn apply(&mut self, event: RevisionEvent) -> Result<(), DeviceDiagnostic> {
        let next = match (self.state.clone(), event) {
            (RevisionState::Draft, RevisionEvent::Validate) => RevisionState::LocallyValidated,
            (RevisionState::LocallyValidated, RevisionEvent::Stage) => RevisionState::Staged,
            (RevisionState::Staged, RevisionEvent::Reject { code }) => {
                RevisionState::Rejected { code }
            }
            (RevisionState::Staged, RevisionEvent::Activate { previous_ready }) => {
                self.previous_ready = previous_ready;
                RevisionState::Active
            }
            (RevisionState::Active, RevisionEvent::Ready) => RevisionState::Ready,
            (RevisionState::Ready, RevisionEvent::Supersede) => RevisionState::Superseded,
            (RevisionState::Active | RevisionState::Ready, RevisionEvent::Fail { code }) => {
                RevisionState::Failed { code }
            }
            (RevisionState::Failed { .. }, RevisionEvent::Rollback) => {
                let restored = self.previous_ready.clone().ok_or_else(|| {
                    invalid_transition(
                        &self.state,
                        &RevisionEvent::Rollback,
                        "no previous ready revision",
                    )
                })?;
                RevisionState::RolledBack { restored }
            }
            (state, RevisionEvent::RestoreFormalPack) if state != RevisionState::Restored => {
                RevisionState::Restored
            }
            (state, event) => {
                return Err(invalid_transition(&state, &event, "event is not allowed"));
            }
        };
        self.state = next;
        Ok(())
    }
}

fn invalid_transition(
    state: &RevisionState,
    event: &RevisionEvent,
    reason: &str,
) -> DeviceDiagnostic {
    DeviceDiagnostic::with_code(
        "device.revision.invalidTransition",
        format!("cannot apply {event:?} while in {state:?}: {reason}"),
    )
}
