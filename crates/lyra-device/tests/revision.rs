use lyra_device::{RevisionEvent, RevisionId, RevisionMachine, RevisionState};

fn revision(hex: char) -> RevisionId {
    RevisionId::from_sha256(&hex.to_string().repeat(64)).expect("revision id")
}

fn staged_machine() -> RevisionMachine {
    let mut machine = RevisionMachine::new(revision('a'));
    machine.apply(RevisionEvent::Validate).expect("validate");
    machine.apply(RevisionEvent::Stage).expect("stage");
    machine
}

#[test]
fn advances_a_revision_through_ready_and_superseded() {
    let mut machine = staged_machine();

    machine
        .apply(RevisionEvent::Activate {
            previous_ready: Some(revision('b')),
        })
        .expect("activate");
    machine.apply(RevisionEvent::Ready).expect("ready");
    assert_eq!(machine.state(), &RevisionState::Ready);
    machine.apply(RevisionEvent::Supersede).expect("supersede");

    assert_eq!(machine.state(), &RevisionState::Superseded);
}

#[test]
fn records_staging_rejection() {
    let mut machine = staged_machine();

    machine
        .apply(RevisionEvent::Reject {
            code: "device.revision.hashMismatch".into(),
        })
        .expect("reject");

    assert_eq!(
        machine.state(),
        &RevisionState::Rejected {
            code: "device.revision.hashMismatch".into()
        }
    );
}

#[test]
fn rolls_a_failed_revision_back_to_the_previous_ready_revision() {
    let previous = revision('b');
    let mut machine = staged_machine();
    machine
        .apply(RevisionEvent::Activate {
            previous_ready: Some(previous.clone()),
        })
        .expect("activate");
    machine.apply(RevisionEvent::Ready).expect("ready");
    machine
        .apply(RevisionEvent::Fail {
            code: "device.runtime.readyTimeout".into(),
        })
        .expect("fail");
    machine.apply(RevisionEvent::Rollback).expect("rollback");

    assert_eq!(
        machine.state(),
        &RevisionState::RolledBack { restored: previous }
    );
}

#[test]
fn invalid_transitions_leave_the_state_unchanged() {
    let mut machine = RevisionMachine::new(revision('a'));

    let diagnostic = machine
        .apply(RevisionEvent::Activate {
            previous_ready: None,
        })
        .expect_err("activate before stage");

    assert_eq!(diagnostic.code, "device.revision.invalidTransition");
    assert_eq!(machine.state(), &RevisionState::Draft);
}

#[test]
fn rollback_requires_a_previous_ready_revision() {
    let mut machine = staged_machine();
    machine
        .apply(RevisionEvent::Activate {
            previous_ready: None,
        })
        .expect("activate");
    machine
        .apply(RevisionEvent::Fail {
            code: "device.runtime.jsError".into(),
        })
        .expect("fail");

    let diagnostic = machine
        .apply(RevisionEvent::Rollback)
        .expect_err("no rollback target");

    assert_eq!(diagnostic.code, "device.revision.invalidTransition");
    assert!(matches!(machine.state(), RevisionState::Failed { .. }));
}

#[test]
fn restore_returns_to_the_formal_pack() {
    let mut machine = staged_machine();
    machine
        .apply(RevisionEvent::RestoreFormalPack)
        .expect("restore");

    assert_eq!(machine.state(), &RevisionState::Restored);
}

#[test]
fn rejects_non_sha256_revision_ids() {
    let diagnostic = RevisionId::from_sha256("ABC").expect_err("invalid hash");
    assert_eq!(diagnostic.code, "device.revision.invalidId");
}
