//! ArtifactAudit集約の保存観測契約。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    ArtifactAudit, ArtifactAuditEvent, ArtifactWriteObservation, HookHealthTarget, IntentDirName,
    SpaceName,
};
fn target(name: &str) -> HookHealthTarget {
    HookHealthTarget::new(
        SpaceName::parse("default").unwrap(),
        Some(IntentDirName::parse(name).unwrap()),
    )
}
#[test]
fn saved_artifacts_are_one_event_each_and_replay_by_the_same_id() {
    let scope = target("260909-artifact");
    let at: DateTime<Utc> = "2026-09-09T01:00:00Z".parse().unwrap();
    let later = at + chrono::Duration::seconds(1);
    let first = ArtifactWriteObservation::new(
        scope.clone(),
        "Write".into(),
        "/p/a.md".into(),
        "construction > code".into(),
        true,
    );
    let (mut audit, created) = ArtifactAudit::start(first.clone(), at).unwrap();
    assert!(matches!(created, ArtifactAuditEvent::Saved(_)));
    assert_eq!(audit.seq_nr(), 1);
    let second = ArtifactWriteObservation::new(
        scope,
        "Edit".into(),
        "/p/a.md".into(),
        "construction > code".into(),
        false,
    );
    let saved = audit.record(second, later).unwrap();
    assert_eq!(audit.seq_nr(), 2);
    assert_eq!(audit.last_tool(), "Edit");
    assert_eq!(
        ArtifactAudit::replay(
            ArtifactAudit::start(first, at).unwrap().0,
            [(saved, 2, later)]
        ),
        audit
    );
    let foreign = ArtifactWriteObservation::new(
        target("260909-other"),
        "Write".into(),
        "/p/b.md".into(),
        "construction".into(),
        true,
    );
    let before_refusal = audit.clone();
    assert!(audit.record(foreign, later).is_err());
    assert_eq!(audit, before_refusal);
}

fn history() -> (ArtifactAudit, ArtifactAuditEvent, DateTime<Utc>) {
    let at = "2026-09-09T01:00:00Z".parse().unwrap();
    let observation = ArtifactWriteObservation::new(
        target("260909-artifact"),
        "Write".into(),
        "/p/a.md".into(),
        "construction".into(),
        true,
    );
    let (base, event) = ArtifactAudit::start(observation, at).unwrap();
    (base, event, at)
}

#[test]
#[should_panic(expected = "artifact audit history sequence")]
fn replay_rejects_a_gap_in_the_saved_sequence() {
    let (base, event, at) = history();
    let _ = ArtifactAudit::replay(base, [(event, 3, at)]);
}

#[test]
#[should_panic(expected = "artifact audit history aggregate")]
fn replay_rejects_an_event_from_another_aggregate() {
    let (base, _, at) = history();
    let observation = ArtifactWriteObservation::new(
        target("260909-other"),
        "Write".into(),
        "/p/b.md".into(),
        "construction".into(),
        true,
    );
    let (_, foreign) = ArtifactAudit::start(observation, at).unwrap();
    let _ = ArtifactAudit::replay(base, [(foreign, 2, at)]);
}

#[test]
#[should_panic(expected = "artifact audit history target")]
fn replay_rejects_a_foreign_target_inside_an_own_event() {
    use core_command_domain::workspace::{ArtifactAuditEventId, ArtifactSaved};
    let (base, _, at) = history();
    let observation = ArtifactWriteObservation::new(
        target("260909-other"),
        "Write".into(),
        "/p/b.md".into(),
        "construction".into(),
        true,
    );
    let event = ArtifactAuditEvent::Saved(ArtifactSaved::new(
        ArtifactAuditEventId::generate(),
        base.id().clone(),
        observation,
    ));
    let _ = ArtifactAudit::replay(base, [(event, 2, at)]);
}

#[test]
fn an_empty_delta_preserves_the_complete_snapshot() {
    let (base, _, _) = history();
    let base = base.with_version(7);
    assert_eq!(ArtifactAudit::replay(base.clone(), []), base);
}

#[test]
fn replay_applies_saved_values_without_changing_the_storage_version() {
    use core_command_domain::workspace::{ArtifactAuditEventId, ArtifactSaved};
    let (base, _, at) = history();
    let base = base.with_version(7);
    let id = base.id().clone();
    let later = at + chrono::Duration::seconds(12);
    let observation = ArtifactWriteObservation::new(
        base.target().clone(),
        "Edit".into(),
        "/p/next.md".into(),
        "construction > changed".into(),
        false,
    );
    let event = ArtifactAuditEvent::Saved(ArtifactSaved::new(
        ArtifactAuditEventId::generate(),
        id.clone(),
        observation,
    ));
    let restored = ArtifactAudit::replay(base, [(event, 2, later)]);
    assert_eq!(restored.id(), &id);
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.version(), 7);
    assert_eq!(restored.last_file(), "/p/next.md");
    assert_eq!(restored.last_tool(), "Edit");
    assert_eq!(restored.last_context(), "construction > changed");
    assert!(!restored.last_created());
    assert_eq!(restored.last_at(), later);
}

#[test]
fn a_full_sequence_refuses_new_commands_without_mutation() {
    use core_command_domain::workspace::{ArtifactAuditRecord, HookHealthError};
    let (base, _, at) = history();
    let mut full = ArtifactAudit::new(
        base.id().clone(),
        base.target().clone(),
        usize::MAX,
        7,
        ArtifactAuditRecord::new(
            "/p/a.md".into(),
            "Write".into(),
            "construction".into(),
            true,
            at,
        ),
    )
    .unwrap();
    let before = full.clone();
    let observation = ArtifactWriteObservation::new(
        base.target().clone(),
        "Edit".into(),
        "/p/b.md".into(),
        "construction".into(),
        false,
    );
    assert_eq!(
        full.record(observation, at),
        Err(HookHealthError::CounterExhausted)
    );
    assert_eq!(full, before);
}
