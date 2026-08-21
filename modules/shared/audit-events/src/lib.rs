//! 監査イベントスキーマ (Published Language) — `EventType` 86 語 22 カテゴリの閉集合・
//! MANDATORY 8・authority deny-list (B5: workspace は行を opaque に扱い、宣言はスキーマ側)。
//!
//! 出典: upstream `aidlc-audit.ts:39-189` (03 §6.5 の完全転記、検算 86/22 済み)。
//! CLI_RESERVED (8) と MERGE_PROTECTED (26+DOCUMENT_*) は as-built 仕様に全列挙が無く、
//! upstream ソース読解 (stage-0 ゴールデン採取) 待ち — 誤推測は audit-merge 互換を壊すため未定義。

#![forbid(unsafe_code)]

macro_rules! event_types {
    ($( $cat:ident { $( $name:ident = $s:literal ),+ $(,)? } )+) => {
        /// 22 カテゴリ (audit-format.md と厳密一致)。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EventCategory { $( $cat, )+ }

        impl EventCategory {
            pub const ALL: &'static [EventCategory] = &[ $( EventCategory::$cat, )+ ];
        }

        /// 86 イベントの閉集合。新イベントの発明は禁止 (E1)。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EventType { $( $( $name, )+ )+ }

        impl EventType {
            pub const ALL: &'static [EventType] = &[ $( $( EventType::$name, )+ )+ ];

            /// 監査行に現れる正準綴り。
            pub fn as_str(self) -> &'static str {
                match self { $( $( EventType::$name => $s, )+ )+ }
            }

            /// 閉集合パース — 未知名は `None` (呼出側が upstream 逐語で拒否する)。
            pub fn parse(s: &str) -> Option<EventType> {
                match s { $( $( $s => Some(EventType::$name), )+ )+ _ => None }
            }

            pub fn category(self) -> EventCategory {
                match self { $( $( EventType::$name => EventCategory::$cat, )+ )+ }
            }
        }
    };
}

event_types! {
    WorkflowLifecycle {
        WorkflowStarted = "WORKFLOW_STARTED", WorkflowCompleted = "WORKFLOW_COMPLETED",
        WorkflowParked = "WORKFLOW_PARKED", WorkflowUnparked = "WORKFLOW_UNPARKED",
    }
    PhaseLifecycle {
        PhaseStarted = "PHASE_STARTED", PhaseCompleted = "PHASE_COMPLETED",
        PhaseVerified = "PHASE_VERIFIED", PhaseSkipped = "PHASE_SKIPPED",
    }
    StageLifecycle {
        StageStarted = "STAGE_STARTED", StageAwaitingApproval = "STAGE_AWAITING_APPROVAL",
        StageRevising = "STAGE_REVISING", StageCompleted = "STAGE_COMPLETED",
        StageJumped = "STAGE_JUMPED", StageSkipped = "STAGE_SKIPPED",
    }
    Session {
        SessionStarted = "SESSION_STARTED", SessionResumed = "SESSION_RESUMED",
        SessionCompacted = "SESSION_COMPACTED", SessionEnded = "SESSION_ENDED",
        HumanTurn = "HUMAN_TURN",
    }
    Initialization {
        WorkspaceScaffolded = "WORKSPACE_SCAFFOLDED", WorkspaceScanned = "WORKSPACE_SCANNED",
        WorkspaceInitialised = "WORKSPACE_INITIALISED",
    }
    Navigation {
        ScopeChanged = "SCOPE_CHANGED", PluginSelectionChanged = "PLUGIN_SELECTION_CHANGED",
        DepthChanged = "DEPTH_CHANGED", TestStrategyChanged = "TEST_STRATEGY_CHANGED",
        ReviewClassChanged = "REVIEW_CLASS_CHANGED", ScopeDetected = "SCOPE_DETECTED",
        Recomposed = "RECOMPOSED",
    }
    Interaction {
        DecisionRecorded = "DECISION_RECORDED", GateApproved = "GATE_APPROVED",
        GateRejected = "GATE_REJECTED", QuestionAnswered = "QUESTION_ANSWERED",
        SummaryConfirmationRecorded = "SUMMARY_CONFIRMATION_RECORDED",
        ReviewRequested = "REVIEW_REQUESTED", ReviewCompleted = "REVIEW_COMPLETED",
        PipelineLinkCompleted = "PIPELINE_LINK_COMPLETED",
    }
    UnitLifecycle {
        UnitStarted = "UNIT_STARTED", UnitPaused = "UNIT_PAUSED",
        UnitResumed = "UNIT_RESUMED", UnitCompleted = "UNIT_COMPLETED",
    }
    Artifact {
        ArtifactCreated = "ARTIFACT_CREATED", ArtifactUpdated = "ARTIFACT_UPDATED",
        ArtifactReused = "ARTIFACT_REUSED",
    }
    Subagent { SubagentCompleted = "SUBAGENT_COMPLETED" }
    ReviewerEnforcement {
        ReviewerScopeBlocked = "REVIEWER_SCOPE_BLOCKED",
        ReviewFreezeBlocked = "REVIEW_FREEZE_BLOCKED",
    }
    PlanApproval { PlanApprovalBlocked = "PLAN_APPROVAL_BLOCKED" }
    Documents {
        DocumentIndexed = "DOCUMENT_INDEXED", DocumentUpdated = "DOCUMENT_UPDATED",
        DocumentRemoved = "DOCUMENT_REMOVED",
    }
    Utility { HealthChecked = "HEALTH_CHECKED" }
    ErrorRecovery { ErrorLogged = "ERROR_LOGGED", RecoveryCompleted = "RECOVERY_COMPLETED" }
    ConstructionBolt {
        BoltStarted = "BOLT_STARTED", BoltCompleted = "BOLT_COMPLETED",
        BoltFailed = "BOLT_FAILED", AutonomyModeSet = "AUTONOMY_MODE_SET",
    }
    Worktree {
        WorktreeCreated = "WORKTREE_CREATED", WorktreeMerged = "WORKTREE_MERGED",
        WorktreeDiscarded = "WORKTREE_DISCARDED", StateForked = "STATE_FORKED",
        StateMerged = "STATE_MERGED", AuditForked = "AUDIT_FORKED", AuditMerged = "AUDIT_MERGED",
    }
    Practices {
        PracticesDiscovered = "PRACTICES_DISCOVERED", PracticesAffirmed = "PRACTICES_AFFIRMED",
        PracticesOverride = "PRACTICES_OVERRIDE", PracticesSectionEmpty = "PRACTICES_SECTION_EMPTY",
    }
    MergeDispatch {
        MergeDispatchInvoked = "MERGE_DISPATCH_INVOKED",
        MergeDispatchReturned = "MERGE_DISPATCH_RETURNED",
        MergeDispatchFallback = "MERGE_DISPATCH_FALLBACK",
    }
    Sensor {
        SensorFired = "SENSOR_FIRED", SensorPassed = "SENSOR_PASSED",
        SensorFailed = "SENSOR_FAILED", SensorBudgetOverride = "SENSOR_BUDGET_OVERRIDE",
        GuardrailLoaded = "GUARDRAIL_LOADED",
    }
    LearningLoop {
        MemoryEmpty = "MEMORY_EMPTY", RuleLearned = "RULE_LEARNED",
        SensorProposed = "SENSOR_PROPOSED",
    }
    Swarm {
        SwarmStarted = "SWARM_STARTED", SwarmUnitConverged = "SWARM_UNIT_CONVERGED",
        SwarmUnitFailed = "SWARM_UNIT_FAILED", SwarmBatonReturned = "SWARM_BATON_RETURNED",
        SwarmCompleted = "SWARM_COMPLETED", SwarmDegraded = "SWARM_DEGRADED",
    }
}

impl EventType {
    /// MANDATORY 8 (レジストリで ✓ 印 — 03 §6.5 M6)。
    pub const MANDATORY: &'static [EventType] = &[
        EventType::WorkflowStarted, EventType::WorkflowCompleted,
        EventType::WorkflowParked, EventType::WorkflowUnparked,
        EventType::PhaseStarted, EventType::PhaseCompleted,
        EventType::StageStarted, EventType::StageCompleted,
    ];

    /// CLI_PROTECTED 18 — 公開 audit CLI からの直接 emit を拒否する authority-bearing
    /// レシート (バイパスは `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1`)。出典: 03 §6.6 L815-819。
    pub const CLI_PROTECTED: &'static [EventType] = &[
        EventType::HumanTurn, EventType::GateApproved, EventType::GateRejected,
        EventType::QuestionAnswered, EventType::AutonomyModeSet,
        EventType::ReviewRequested, EventType::ReviewCompleted,
        EventType::PipelineLinkCompleted, EventType::ArtifactReused,
        EventType::SwarmStarted, EventType::SwarmUnitConverged,
        EventType::UnitStarted, EventType::UnitPaused,
        EventType::UnitResumed, EventType::UnitCompleted,
        EventType::DocumentIndexed, EventType::DocumentUpdated, EventType::DocumentRemoved,
    ];

    pub fn is_mandatory(self) -> bool {
        Self::MANDATORY.contains(&self)
    }

    pub fn is_cli_protected(self) -> bool {
        Self::CLI_PROTECTED.contains(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn closed_set_has_exactly_86_events_in_22_categories() {
        assert_eq!(EventType::ALL.len(), 86);
        assert_eq!(EventCategory::ALL.len(), 22);
    }

    #[test]
    fn category_sizes_match_the_upstream_registry() {
        let mut by_cat: HashMap<&'static str, usize> = HashMap::new();
        for e in EventType::ALL {
            *by_cat.entry(cat_name(e.category())).or_default() += 1;
        }
        let expected = [
            ("WorkflowLifecycle", 4), ("PhaseLifecycle", 4), ("StageLifecycle", 6),
            ("Session", 5), ("Initialization", 3), ("Navigation", 7), ("Interaction", 8),
            ("UnitLifecycle", 4), ("Artifact", 3), ("Subagent", 1), ("ReviewerEnforcement", 2),
            ("PlanApproval", 1), ("Documents", 3), ("Utility", 1), ("ErrorRecovery", 2),
            ("ConstructionBolt", 4), ("Worktree", 7), ("Practices", 4), ("MergeDispatch", 3),
            ("Sensor", 5), ("LearningLoop", 3), ("Swarm", 6),
        ];
        for (name, n) in expected {
            assert_eq!(by_cat.get(name), Some(&n), "category {name}");
        }
    }

    fn cat_name(c: EventCategory) -> &'static str {
        match c {
            EventCategory::WorkflowLifecycle => "WorkflowLifecycle",
            EventCategory::PhaseLifecycle => "PhaseLifecycle",
            EventCategory::StageLifecycle => "StageLifecycle",
            EventCategory::Session => "Session",
            EventCategory::Initialization => "Initialization",
            EventCategory::Navigation => "Navigation",
            EventCategory::Interaction => "Interaction",
            EventCategory::UnitLifecycle => "UnitLifecycle",
            EventCategory::Artifact => "Artifact",
            EventCategory::Subagent => "Subagent",
            EventCategory::ReviewerEnforcement => "ReviewerEnforcement",
            EventCategory::PlanApproval => "PlanApproval",
            EventCategory::Documents => "Documents",
            EventCategory::Utility => "Utility",
            EventCategory::ErrorRecovery => "ErrorRecovery",
            EventCategory::ConstructionBolt => "ConstructionBolt",
            EventCategory::Worktree => "Worktree",
            EventCategory::Practices => "Practices",
            EventCategory::MergeDispatch => "MergeDispatch",
            EventCategory::Sensor => "Sensor",
            EventCategory::LearningLoop => "LearningLoop",
            EventCategory::Swarm => "Swarm",
        }
    }

    #[test]
    fn parse_round_trips_every_event_and_rejects_unknown_names() {
        for e in EventType::ALL {
            assert_eq!(EventType::parse(e.as_str()), Some(*e));
        }
        assert_eq!(EventType::parse("STAGE_DONE"), None);
        assert_eq!(EventType::parse("Requirements Analysis Complete"), None);
    }

    #[test]
    fn mandatory_is_exactly_the_eight_checked_events() {
        assert_eq!(EventType::MANDATORY.len(), 8);
        assert!(EventType::StageCompleted.is_mandatory());
        assert!(!EventType::PhaseVerified.is_mandatory());
    }

    #[test]
    fn cli_protected_is_exactly_eighteen_authority_receipts() {
        assert_eq!(EventType::CLI_PROTECTED.len(), 18);
        assert!(EventType::HumanTurn.is_cli_protected());
        assert!(!EventType::StageCompleted.is_cli_protected());
    }
}
