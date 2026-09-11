//! 指示のセッションと文脈世代を、保存境界で失わない契約。
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use core_command_domain::orchestration::{DirectivePublication, PublishedDirective};
use core_command_domain::workflow_definition::StageSlug;
use core_command_interface_adapter::orchestration::IntentExecutionDto;

#[test]
fn a_reconstructed_directive_retains_its_owner_and_context_epochs() {
    let (mut execution, _) = support::genesis();
    execution
        .issue_directive(
            &DirectivePublication::new(
                "a".repeat(64),
                "b".repeat(64),
                PublishedDirective::RunStage {
                    stage: StageSlug::parse("stage-1").unwrap(),
                    unit: None,
                },
            ),
            support::at(),
        )
        .unwrap();
    let mut document = snapshot_document(&execution);
    let directive = document
        .get_mut("active_directive")
        .unwrap()
        .as_object_mut()
        .unwrap();
    directive.insert(
        "owner_session".into(),
        serde_json::Value::from("owner-session"),
    );
    directive.insert("owner_epoch".into(), serde_json::Value::from(2));
    directive.insert("context_epoch".into(), serde_json::Value::from(3));
    let decoded: IntentExecutionDto = serde_json::from_value(document).unwrap();
    let recovered = decoded.to_domain().unwrap();
    let persisted = snapshot_document(&recovered);
    assert_eq!(
        persisted
            .get("active_directive")
            .unwrap()
            .get("owner_session")
            .unwrap(),
        "owner-session"
    );
    assert_eq!(
        persisted
            .get("active_directive")
            .unwrap()
            .get("owner_epoch")
            .unwrap(),
        2
    );
    assert_eq!(
        persisted
            .get("active_directive")
            .unwrap()
            .get("context_epoch")
            .unwrap(),
        3
    );
}

#[test]
fn a_plan_authority_fingerprint_changes_with_the_context_or_owner_epoch() {
    use core_command_domain::orchestration::{
        ActiveDirective, CodeGenerationAuthority, CodeGenerationRunFloor,
    };
    let publication = DirectivePublication::new(
        "a".repeat(64),
        "b".repeat(64),
        PublishedDirective::RunStage {
            stage: StageSlug::parse("code-generation").unwrap(),
            unit: None,
        },
    );
    let epoch = |owner, context| {
        let directive = ActiveDirective::new(
            1,
            support::intent_id(),
            publication.clone(),
            "b".repeat(64),
            "session".into(),
            owner,
            context,
            1,
        );
        CodeGenerationAuthority::resolve(
            Some(&directive),
            Some(&CodeGenerationRunFloor::new(0, 0, 0, 0, None).unwrap()),
            None,
            Some(&"b".repeat(64)),
        )
        .unwrap()
        .directive_epoch()
        .to_string()
    };
    assert_ne!(epoch(0, 0), epoch(0, 1));
    assert_ne!(epoch(0, 0), epoch(1, 0));
}

#[test]
fn matching_compaction_revokes_authority_and_clears_a_steering_token() {
    use core_command_domain::orchestration::{
        CodeGenerationAuthority, CodeGenerationRunFloor, DirectiveContextInvalidation,
        PlanApprovalOperationId,
    };
    let (mut execution, _) = support::genesis();
    let publication = DirectivePublication::new(
        "a".repeat(64),
        "b".repeat(64),
        PublishedDirective::LoadSteering {
            stage: StageSlug::parse("code-generation").unwrap(),
            part: 1,
            parts: 2,
            token: "old-token".into(),
        },
    );
    execution
        .issue_directive(&publication, support::at())
        .unwrap();
    let request = DirectiveContextInvalidation::new(
        execution.intent_id().clone(),
        "a".repeat(64),
        "b".repeat(64),
        "sessionless:aaaaaaaaaaaaaaaa".into(),
    );
    let before = execution.seq_nr();
    execution
        .invalidate_directive_context(
            &PlanApprovalOperationId::generate(),
            &request,
            support::at(),
        )
        .unwrap();
    assert_eq!(execution.seq_nr(), before + 1);
    let directive = execution.active_directive().unwrap();
    assert!(matches!(
        directive.directive(),
        PublishedDirective::Error { .. }
    ));
    assert_eq!(directive.context_epoch(), 1);
    assert!(
        CodeGenerationAuthority::resolve(
            Some(directive),
            Some(&CodeGenerationRunFloor::new(0, 0, 0, 0, None).unwrap()),
            None,
            Some(&"b".repeat(64))
        )
        .is_err()
    );
    execution
        .invalidate_directive_context(
            &PlanApprovalOperationId::generate(),
            &request,
            support::at(),
        )
        .unwrap();
    assert_eq!(execution.active_directive().unwrap().context_epoch(), 2);
    execution
        .issue_directive(&publication, support::at())
        .unwrap();
    let current = execution.active_directive().unwrap();
    assert_eq!(current.context_epoch(), 2);
    assert_eq!(current.issuance_revision(), current.revision());
}

#[test]
fn exhausted_context_or_revision_cannot_partially_invalidate_a_directive() {
    use core_command_domain::orchestration::{
        DirectiveContextInvalidation, PlanApprovalOperationId,
    };
    for field in ["revision", "context_epoch"] {
        let (mut execution, _) = support::genesis();
        execution
            .issue_directive(
                &DirectivePublication::new(
                    "a".repeat(64),
                    "b".repeat(64),
                    PublishedDirective::RunStage {
                        stage: StageSlug::parse("code-generation").unwrap(),
                        unit: None,
                    },
                ),
                support::at(),
            )
            .unwrap();
        let mut document = snapshot_document(&execution);
        document
            .get_mut("active_directive")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::Value::from(u64::MAX));
        let decoded: IntentExecutionDto = serde_json::from_value(document).unwrap();
        let mut execution = decoded.to_domain().unwrap();
        let before = execution.clone();
        let request = DirectiveContextInvalidation::new(
            execution.intent_id().clone(),
            "a".repeat(64),
            "b".repeat(64),
            "sessionless:aaaaaaaaaaaaaaaa".into(),
        );
        assert!(
            execution
                .invalidate_directive_context(
                    &PlanApprovalOperationId::generate(),
                    &request,
                    support::at()
                )
                .is_err()
        );
        assert_eq!(execution, before, "{field}");
    }
}

fn snapshot_document(
    execution: &core_command_domain::orchestration::IntentExecution,
) -> serde_json::Value {
    use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
    let value = to_value(&IntentExecutionDto::of(execution)).unwrap();
    serde_json::from_str(&serialize(&value, SerializationProfile::ContractCompact)).unwrap()
}
