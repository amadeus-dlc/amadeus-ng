//! フック観測の所有と、記録先を取り違えないための契約。
use core_command_domain::workspace::{HookHealthTarget, IntentDirName, SpaceName};
#[test]
fn observation_targets_keep_cold_space_and_each_record_separate() {
    let cold = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let record = HookHealthTarget::new(
        SpaceName::parse("default").unwrap(),
        Some(IntentDirName::parse("260908-hook-health").unwrap()),
    );
    let other = HookHealthTarget::new(SpaceName::parse("other").unwrap(), None);
    assert_eq!(cold.relative_directory(), "spaces/default/intents");
    assert_eq!(
        record.relative_directory(),
        "spaces/default/intents/260908-hook-health"
    );
    assert_eq!(other.relative_directory(), "spaces/other/intents");
    assert_ne!(cold, record);
    assert_ne!(cold, other);
}

#[test]
fn hook_names_cannot_escape_the_observation_file_namespace() {
    use core_command_domain::workspace::HookName;
    for name in [
        "write-audit-log",
        "record-human-turn",
        "stop-forwarding-loop",
    ] {
        assert_eq!(HookName::parse(name).unwrap().as_str(), name);
    }
    for invalid in [
        "",
        "../write-audit-log",
        "a/b",
        "a\\b",
        "A",
        " write-audit-log",
        "a\n",
        "a\0",
    ] {
        assert!(HookName::parse(invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn health_identity_separates_hooks_spaces_and_records_and_round_trips() {
    use core_command_domain::workspace::{HookHealthId, HookName};
    let cold = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let record = HookHealthTarget::new(
        SpaceName::parse("default").unwrap(),
        Some(IntentDirName::parse("260908-hook-health").unwrap()),
    );
    let other = HookHealthTarget::new(SpaceName::parse("other").unwrap(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let id = HookHealthId::for_hook(&cold, &hook);
    assert_eq!(id, HookHealthId::for_hook(&cold, &hook));
    assert_ne!(id, HookHealthId::for_hook(&record, &hook));
    assert_ne!(id, HookHealthId::for_hook(&other, &hook));
    assert_ne!(
        id,
        HookHealthId::for_hook(&cold, &HookName::parse("record-human-turn").unwrap())
    );
    assert_eq!(HookHealthId::parse(id.as_str()).unwrap(), id);
    for invalid in [
        "workspace".to_string(),
        "hook-health:".to_string(),
        format!("hook-health:{}", "G".repeat(64)),
        format!(" {}", id.as_str()),
    ] {
        assert!(HookHealthId::parse(&invalid).is_err());
    }
}

#[test]
fn the_first_hook_invocation_creates_one_owned_heartbeat_fact() {
    use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookHealthId, HookName};
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let (health, event) = HookHealth::start(target.clone(), hook.clone(), at).unwrap();
    assert_eq!(health.id(), &HookHealthId::for_hook(&target, &hook));
    assert_eq!(event.aggregate_id(), health.id());
    assert_eq!(health.target(), &target);
    assert_eq!(health.hook(), &hook);
    assert_eq!(health.heartbeat(), Some(at));
    assert_eq!(health.seq_nr(), 1);
    assert_eq!(health.version(), 0);
    assert!(matches!(event, HookHealthEvent::Started(_)));
    if let HookHealthEvent::Started(started) = &event {
        assert_eq!(started.target(), &target);
        assert_eq!(started.hook(), &hook);
    }
    assert_ne!(event.id().as_str(), health.id().as_str());
}

#[test]
fn subsequent_heartbeats_are_one_event_each_and_replay_the_same_state() {
    use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookName};
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let later = at + chrono::Duration::seconds(3);
    let (mut health, created) =
        HookHealth::start(target, HookName::parse("write-audit-log").unwrap(), at).unwrap();
    let snapshot = health.clone();
    let observed = health.observe_heartbeat(later).unwrap();
    assert!(matches!(observed, HookHealthEvent::HeartbeatObserved(_)));
    assert_eq!(observed.aggregate_id(), health.id());
    assert_ne!(observed.id(), created.id());
    assert_eq!(health.heartbeat(), Some(later));
    assert_eq!(health.seq_nr(), 2);
    assert_eq!(HookHealth::replay(snapshot, [(observed, 2, later)]), health);
}

#[test]
fn audit_drops_are_sanitized_and_counted_without_touching_heartbeat() {
    use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookName};
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-08T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let later = at + chrono::Duration::seconds(1);
    let (mut health, _) =
        HookHealth::start(target, HookName::parse("write-audit-log").unwrap(), at).unwrap();
    let event = health.record_drop("EISDIR\r\nwrite failed", later).unwrap();
    assert!(matches!(event, HookHealthEvent::AuditDropped(_)));
    assert_eq!(health.drops(), 1);
    assert_eq!(
        health.latest_drop().unwrap().as_str(),
        "EISDIR write failed"
    );
    assert_eq!(health.heartbeat(), Some(at));
    assert_eq!(health.seq_nr(), 2);
    assert!(health.record_drop("", later).is_err());
}

#[test]
fn first_drop_and_later_heartbeat_keep_distinct_observation_times() {
    use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookName};
    let target = HookHealthTarget::new(SpaceName::default(), None);
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-09T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let (mut health, first) = HookHealth::start_with_drop(
        target,
        HookName::parse("session-end").unwrap(),
        "unknown stamp",
        at,
    )
    .unwrap();
    assert!(matches!(first, HookHealthEvent::FirstDropObserved(_)));
    assert_eq!(health.heartbeat(), None);
    assert_eq!(health.observed_at(), at);
    assert_eq!(health.seq_nr(), 1);
    assert_eq!(health.drops(), 1);
    let base = health.clone();
    let later = at + chrono::Duration::seconds(3);
    let heartbeat = health.observe_heartbeat(later).unwrap();
    let dropped_at = later + chrono::Duration::seconds(5);
    let drop = health.record_drop("second", dropped_at).unwrap();
    assert_eq!(health.heartbeat(), Some(later));
    assert_eq!(health.observed_at(), dropped_at);
    assert_eq!(health.drops(), 2);
    assert_eq!(
        HookHealth::replay(base, [(heartbeat, 2, later), (drop, 3, dropped_at)]),
        health
    );
}

#[test]
fn a_first_drop_requires_a_real_failure_reason() {
    use core_command_domain::workspace::{HookHealth, HookName};
    let target = HookHealthTarget::new(SpaceName::default(), None);
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-09T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(
        HookHealth::start_with_drop(target, HookName::parse("session-end").unwrap(), " \n ", at)
            .is_err()
    );
}

#[test]
#[should_panic(expected = "invalid HookHealth history")]
fn replaying_hook_health_with_a_missing_sequence_is_fatal() {
    use core_command_domain::workspace::{HookHealth, HookName};
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let at = chrono::DateTime::parse_from_rfc3339("2026-09-09T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let (mut health, _) =
        HookHealth::start(target, HookName::parse("write-audit-log").unwrap(), at).unwrap();
    let snapshot = health.clone();
    let event = health.observe_heartbeat(at).unwrap();
    let _ = HookHealth::replay(snapshot, [(event, 3, at)]);
}
