//! 停止制御の進捗・再入・履歴の契約。
#![allow(clippy::unwrap_used)]
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationRequest, ContinuationSignature, IntentExecutionId,
    WorkflowContinuation, WorkflowContinuationId,
};

fn request(signature: char, reentrant: bool, limit: u64) -> ContinuationRequest {
    ContinuationRequest::new(
        ContinuationAttemptId::generate(),
        Some(
            ContinuationSignature::parse(&format!(
                "code-generation::{}::{}",
                signature.to_string().repeat(64),
                "b".repeat(64)
            ))
            .unwrap(),
        ),
        reentrant,
        limit,
    )
    .unwrap()
}
fn id() -> WorkflowContinuationId {
    WorkflowContinuationId::for_execution(
        &IntentExecutionId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").unwrap(),
    )
}
fn time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-09-09T01:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

#[test]
fn an_unpublished_decision_cannot_be_used_as_the_next_counter() {
    let (mut continuation, _) =
        WorkflowContinuation::start(id(), request('a', false, 2), time()).unwrap();
    assert!(
        continuation
            .consider(request('a', false, 2), time())
            .is_err()
    );
}

#[test]
fn a_genesis_snapshot_cannot_claim_that_publication_was_already_confirmed() {
    let request = request('a', false, 3);
    let before =
        core_command_domain::orchestration::ContinuationGuard::new(None, 0, false).unwrap();
    let selected = before.after(&request).unwrap();
    assert!(
        WorkflowContinuation::new(
            id(),
            request,
            core_command_domain::orchestration::ContinuationCounter::new(
                before,
                selected,
                Some(true)
            ),
            1,
            0,
            time()
        )
        .is_err()
    );
}

#[test]
fn a_failed_publication_keeps_the_observed_counter_and_rejects_stale_settlement() {
    let (mut continuation, first) =
        WorkflowContinuation::start(id(), request('a', false, 3), time()).unwrap();
    let selected = continuation.clone();
    let failed = continuation
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                first.request().id().clone(),
                false,
            ),
            time(),
        )
        .unwrap();
    assert_eq!(
        WorkflowContinuation::replay(selected, [(failed, 2, time())]),
        continuation
    );
    assert!(!continuation.guard().is_initialized());
    let next = continuation
        .consider(request('a', false, 3), time())
        .unwrap();
    assert_eq!(next.count(), 1);
    let before_stale = continuation.clone();
    assert!(
        continuation
            .record_publication(
                &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                    first.request().id().clone(),
                    false
                ),
                time()
            )
            .is_err()
    );
    assert_eq!(continuation, before_stale);
    continuation
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                next.request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    assert!(
        continuation
            .record_publication(
                &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                    next.request().id().clone(),
                    false
                ),
                time()
            )
            .is_err()
    );
}

#[test]
fn an_unreadable_public_counter_is_a_new_observation_even_after_successful_publication() {
    let (mut continuation, first) =
        WorkflowContinuation::start(id(), request('a', false, 3), time()).unwrap();
    continuation
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                first.request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    let next = continuation
        .consider(
            request('a', false, 3).with_observed_guard(
                core_command_domain::orchestration::ContinuationGuard::new(None, 0, false).unwrap(),
            ),
            time(),
        )
        .unwrap();
    assert_eq!(next.count(), 1);
}

#[test]
fn first_and_repeated_stops_reach_the_limit_without_new_progress() {
    let (mut continuation, event) =
        WorkflowContinuation::start(id(), request('a', false, 2), time()).unwrap();
    assert_eq!(event.count(), 1);
    assert!(event.blocked());
    continuation
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                continuation.last_request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    let next = continuation
        .consider(request('a', false, 2), time())
        .unwrap();
    assert_eq!(next.count(), 2);
    assert!(!next.blocked());
    assert_ne!(event.id(), next.id());
    assert_ne!(event.request().id().as_str(), event.id().as_str());
}

#[test]
fn progress_resets_a_reentrant_sequence_even_after_the_previous_limit() {
    let (mut continuation, _) =
        WorkflowContinuation::start(id(), request('a', true, 2), time()).unwrap();
    continuation
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                continuation.last_request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    let next = continuation
        .consider(request('c', true, 2), time())
        .unwrap();
    assert_eq!(next.count(), 1);
    assert!(next.blocked());
}

#[test]
fn the_first_reentrant_stop_starts_at_two() {
    let (_, event) = WorkflowContinuation::start(id(), request('a', true, 2), time()).unwrap();
    assert_eq!(event.count(), 2);
    assert!(!event.blocked());
}

#[test]
fn positive_one_releases_immediately_and_eight_keeps_pending_work_running() {
    let (_, immediate) = WorkflowContinuation::start(id(), request('a', false, 1), time()).unwrap();
    assert!(!immediate.blocked());
    let (mut long, _) = WorkflowContinuation::start(id(), request('a', false, 8), time()).unwrap();
    for count in 2..=8 {
        long.record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                long.last_request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
        let event = long.consider(request('a', false, 8), time()).unwrap();
        assert_eq!(event.count(), count);
        assert_eq!(event.blocked(), count < 8);
    }
}

#[test]
fn a_snapshot_and_only_its_later_events_replay_the_same_state() {
    let (mut actual, _) =
        WorkflowContinuation::start(id(), request('a', false, 3), time()).unwrap();
    actual
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                actual.last_request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    let snapshot = actual.clone();
    let event = actual.consider(request('a', false, 3), time()).unwrap();
    assert_eq!(
        WorkflowContinuation::replay(snapshot, [(event, 3, time())]),
        actual
    );
}

#[test]
fn a_genesis_snapshot_cannot_claim_an_unobserved_streak() {
    assert!(
        WorkflowContinuation::new(
            id(),
            request('a', false, 3),
            core_command_domain::orchestration::ContinuationCounter::new(
                core_command_domain::orchestration::ContinuationGuard::new(None, 0, false).unwrap(),
                core_command_domain::orchestration::ContinuationGuard::new(
                    request('a', false, 3).signature().cloned(),
                    9,
                    true
                )
                .unwrap(),
                None
            ),
            1,
            0,
            time()
        )
        .is_err()
    );
}

#[test]
#[should_panic(expected = "invalid WorkflowContinuation history")]
fn a_missing_sequence_in_saved_history_is_fatal() {
    let (mut actual, _) =
        WorkflowContinuation::start(id(), request('a', false, 3), time()).unwrap();
    actual
        .record_publication(
            &core_command_domain::orchestration::ContinuationPublicationObservation::new(
                actual.last_request().id().clone(),
                true,
            ),
            time(),
        )
        .unwrap();
    let snapshot = actual.clone();
    let event = actual.consider(request('a', false, 3), time()).unwrap();
    let _ = WorkflowContinuation::replay(snapshot, [(event, 4, time())]);
}

#[test]
fn progress_signatures_match_the_fixed_upstream_function_for_all_observation_fields() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/selfhost-stage1/stop-values.json"
    ))
    .unwrap();
    let observations = corpus.get("observations").unwrap().as_array().unwrap();
    assert!(!observations.is_empty());
    let mut ids = std::collections::BTreeSet::new();
    for case in observations {
        let id = case.get("id").unwrap().as_str().unwrap();
        assert!(ids.insert(id));
        let directive =
            core_infrastructure::canon_json::to_value(case.get("directive").unwrap()).unwrap();
        let signature = ContinuationSignature::from_observation(
            case.get("state").unwrap().as_str().unwrap(),
            &directive,
        )
        .unwrap();
        assert_eq!(
            signature.as_str(),
            case.get("signature").unwrap().as_str().unwrap(),
            "{id}"
        );
    }
}

#[test]
fn reset_requests_cannot_also_claim_to_wait_for_a_question() {
    let reset =
        ContinuationRequest::new(ContinuationAttemptId::generate(), None, false, 1).unwrap();
    let request = reset.with_wait(Some(
        core_command_domain::orchestration::ContinuationWait::Question,
    ));
    assert!(request.is_err(), "resetと待機を同時に表現しない");
}

#[test]
fn reset_requests_cannot_also_claim_to_probe_for_a_wait() {
    let reset =
        ContinuationRequest::new(ContinuationAttemptId::generate(), None, false, 1).unwrap();
    assert!(reset.with_wait_probe().is_err());
}

#[test]
fn the_guard_signature_is_the_progress_that_last_moved_the_counter() {
    let (started, _) = WorkflowContinuation::start(id(), request('a', false, 2), time()).unwrap();
    assert_eq!(
        started.guard_signature(),
        started.last_request().signature(),
        "開始直後の実効 guard は開始要求の進捗を指す"
    );
    assert_eq!(started.guard_signature(), started.guard().signature());
}
