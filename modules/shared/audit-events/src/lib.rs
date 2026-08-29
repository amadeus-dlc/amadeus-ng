//! 監査イベントスキーマ (Published Language) — `EventType` 86 語 22 カテゴリの閉集合・
//! MANDATORY 8・authority deny-list (B5: workspace は行を opaque に扱い、宣言はスキーマ側)。
//!
//! 出典: upstream `aidlc-audit.ts:39-189` (03 §6.5 の完全転記、検算 86/22 済み)。
//! CLI_RESERVED (8) と MERGE_PROTECTED (26+DOCUMENT_*) は as-built 仕様に全列挙が無く、
//! upstream ソース読解 (stage-0 ゴールデン採取) 待ち — 誤推測は audit-merge 互換を壊すため未定義。

#![forbid(unsafe_code)]

macro_rules! event_types {
    ($( $cat:ident { $( $name:ident = $s:literal => $h:literal ),+ $(,)? } )+) => {
        /// 22 カテゴリ (audit-format.md と厳密一致)。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EventCategory { $(
            /// upstream レジストリの分類見出しに対応するカテゴリ。所属イベントは
            /// `EventType::ALL` を `category()` で絞り込んで得る。
            $cat,
        )+ }

        impl EventCategory {
            /// 22 カテゴリの全値。並びは upstream レジストリの掲載順 (完全転記なので
            /// 宣言順 = 掲載順)。
            pub const ALL: &'static [EventCategory] = &[ $( EventCategory::$cat, )+ ];
        }

        /// 86 イベントの閉集合。新イベントの発明は禁止 (E1)。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EventType { $( $(
            /// 監査行の event 名 1 語。ワイヤ綴りは `as_str`、所属カテゴリは `category`。
            $name,
        )+ )+ }

        impl EventType {
            /// 86 イベントの全値。並びは upstream レジストリの掲載順 (カテゴリ順 ×
            /// カテゴリ内掲載順)。
            pub const ALL: &'static [EventType] = &[ $( $( EventType::$name, )+ )+ ];

            /// 監査行に現れる正準綴り。
            pub const fn as_str(self) -> &'static str {
                match self { $( $( EventType::$name => $s, )+ )+ }
            }

            /// 閉集合パース — 未知名は `None` (呼出側が upstream 逐語で拒否する)。
            pub fn parse(s: &str) -> Option<EventType> {
                match s { $( $( $s => Some(EventType::$name), )+ )+ _ => None }
            }

            /// レジストリ上でこのイベントが載っている分類。全域関数 (未分類は存在しない)。
            pub const fn category(self) -> EventCategory {
                match self { $( $( EventType::$name => EventCategory::$cat, )+ )+ }
            }

            /// 監査ブロックの `## ` 見出しに書く逐語文字列 (upstream `EVENT_HEADINGS`)。
            ///
            /// **語形はワイヤ綴りから機械変換できない** — `STAGE_COMPLETED` は
            /// `Stage Completion` (名詞化) なのに `UNIT_COMPLETED` は `Unit Completed`
            /// (過去分詞) であり、`RECOMPOSED` にいたっては語幹に無い語が付いて
            /// `Plan Recomposed` になる。したがって 86 件は 1 件ずつ逐語で持つ。
            ///
            /// upstream の `EVENT_HEADINGS[x] || x` というフォールバックはここには無い —
            /// 閉集合の 86 語すべてに見出しがあり (実測)、`EventType` は閉集合なので、
            /// フォールバックが要る「非 taxonomy 名」はそもそも構成できない。
            pub const fn heading(self) -> &'static str {
                match self { $( $( EventType::$name => $h, )+ )+ }
            }
        }
    };
}

event_types! {
    WorkflowLifecycle {
        WorkflowStarted = "WORKFLOW_STARTED" => "Workflow Start", WorkflowCompleted = "WORKFLOW_COMPLETED" => "Workflow Completion",
        WorkflowParked = "WORKFLOW_PARKED" => "Workflow Parked", WorkflowUnparked = "WORKFLOW_UNPARKED" => "Workflow Unparked",
    }
    PhaseLifecycle {
        PhaseStarted = "PHASE_STARTED" => "Phase Start", PhaseCompleted = "PHASE_COMPLETED" => "Phase Completion",
        PhaseVerified = "PHASE_VERIFIED" => "Phase Verification", PhaseSkipped = "PHASE_SKIPPED" => "Phase Skip",
    }
    StageLifecycle {
        StageStarted = "STAGE_STARTED" => "Stage Start", StageAwaitingApproval = "STAGE_AWAITING_APPROVAL" => "Stage Awaiting Approval",
        StageRevising = "STAGE_REVISING" => "Stage Revising", StageCompleted = "STAGE_COMPLETED" => "Stage Completion",
        StageJumped = "STAGE_JUMPED" => "Stage Jump", StageSkipped = "STAGE_SKIPPED" => "Stage Skip",
    }
    Session {
        SessionStarted = "SESSION_STARTED" => "Session Start", SessionResumed = "SESSION_RESUMED" => "Session Resume",
        SessionCompacted = "SESSION_COMPACTED" => "Session Compacted", SessionEnded = "SESSION_ENDED" => "Session End",
        HumanTurn = "HUMAN_TURN" => "Human Turn",
    }
    Initialization {
        WorkspaceScaffolded = "WORKSPACE_SCAFFOLDED" => "Workspace Scaffolded", WorkspaceScanned = "WORKSPACE_SCANNED" => "Workspace Scanned",
        WorkspaceInitialised = "WORKSPACE_INITIALISED" => "Workspace Initialised",
    }
    Navigation {
        ScopeChanged = "SCOPE_CHANGED" => "Scope Change", PluginSelectionChanged = "PLUGIN_SELECTION_CHANGED" => "Plugin Selection Change",
        DepthChanged = "DEPTH_CHANGED" => "Depth Change", TestStrategyChanged = "TEST_STRATEGY_CHANGED" => "Test Strategy Change",
        ReviewClassChanged = "REVIEW_CLASS_CHANGED" => "Review Class Change", ScopeDetected = "SCOPE_DETECTED" => "Scope Detection",
        Recomposed = "RECOMPOSED" => "Plan Recomposed",
    }
    Interaction {
        DecisionRecorded = "DECISION_RECORDED" => "Decision Recorded", GateApproved = "GATE_APPROVED" => "Gate Approved",
        GateRejected = "GATE_REJECTED" => "Gate Rejected", QuestionAnswered = "QUESTION_ANSWERED" => "Question Answered",
        SummaryConfirmationRecorded = "SUMMARY_CONFIRMATION_RECORDED" => "Summary Confirmation Recorded",
        ReviewRequested = "REVIEW_REQUESTED" => "Review Requested", ReviewCompleted = "REVIEW_COMPLETED" => "Review Completed",
        PipelineLinkCompleted = "PIPELINE_LINK_COMPLETED" => "Pipeline Link Completed",
    }
    UnitLifecycle {
        UnitStarted = "UNIT_STARTED" => "Unit Started", UnitPaused = "UNIT_PAUSED" => "Unit Paused",
        UnitResumed = "UNIT_RESUMED" => "Unit Resumed", UnitCompleted = "UNIT_COMPLETED" => "Unit Completed",
    }
    Artifact {
        ArtifactCreated = "ARTIFACT_CREATED" => "Artifact Created", ArtifactUpdated = "ARTIFACT_UPDATED" => "Artifact Updated",
        ArtifactReused = "ARTIFACT_REUSED" => "Artifact Reused",
    }
    Subagent { SubagentCompleted = "SUBAGENT_COMPLETED" => "Subagent Completed" }
    ReviewerEnforcement {
        ReviewerScopeBlocked = "REVIEWER_SCOPE_BLOCKED" => "Reviewer Scope Blocked",
        ReviewFreezeBlocked = "REVIEW_FREEZE_BLOCKED" => "Review Freeze Blocked",
    }
    PlanApproval { PlanApprovalBlocked = "PLAN_APPROVAL_BLOCKED" => "Plan Approval Blocked" }
    Documents {
        DocumentIndexed = "DOCUMENT_INDEXED" => "Document Indexed", DocumentUpdated = "DOCUMENT_UPDATED" => "Document Updated",
        DocumentRemoved = "DOCUMENT_REMOVED" => "Document Removed",
    }
    Utility { HealthChecked = "HEALTH_CHECKED" => "Health Check" }
    ErrorRecovery { ErrorLogged = "ERROR_LOGGED" => "Error Logged", RecoveryCompleted = "RECOVERY_COMPLETED" => "Recovery Completed" }
    ConstructionBolt {
        BoltStarted = "BOLT_STARTED" => "Bolt Started", BoltCompleted = "BOLT_COMPLETED" => "Bolt Completed",
        BoltFailed = "BOLT_FAILED" => "Bolt Failed", AutonomyModeSet = "AUTONOMY_MODE_SET" => "Autonomy Mode Set",
    }
    Worktree {
        WorktreeCreated = "WORKTREE_CREATED" => "Worktree Created", WorktreeMerged = "WORKTREE_MERGED" => "Worktree Merged",
        WorktreeDiscarded = "WORKTREE_DISCARDED" => "Worktree Discarded", StateForked = "STATE_FORKED" => "State Forked",
        StateMerged = "STATE_MERGED" => "State Merged", AuditForked = "AUDIT_FORKED" => "Audit Forked", AuditMerged = "AUDIT_MERGED" => "Audit Merged",
    }
    Practices {
        PracticesDiscovered = "PRACTICES_DISCOVERED" => "Practices Discovered", PracticesAffirmed = "PRACTICES_AFFIRMED" => "Practices Affirmed",
        PracticesOverride = "PRACTICES_OVERRIDE" => "Practices Override", PracticesSectionEmpty = "PRACTICES_SECTION_EMPTY" => "Practices Section Empty",
    }
    MergeDispatch {
        MergeDispatchInvoked = "MERGE_DISPATCH_INVOKED" => "Merge Dispatch Invoked",
        MergeDispatchReturned = "MERGE_DISPATCH_RETURNED" => "Merge Dispatch Returned",
        MergeDispatchFallback = "MERGE_DISPATCH_FALLBACK" => "Merge Dispatch Fallback",
    }
    Sensor {
        SensorFired = "SENSOR_FIRED" => "Sensor Fired", SensorPassed = "SENSOR_PASSED" => "Sensor Passed",
        SensorFailed = "SENSOR_FAILED" => "Sensor Failed", SensorBudgetOverride = "SENSOR_BUDGET_OVERRIDE" => "Sensor Budget Override",
        GuardrailLoaded = "GUARDRAIL_LOADED" => "Guardrail Loaded",
    }
    LearningLoop {
        MemoryEmpty = "MEMORY_EMPTY" => "Memory Empty", RuleLearned = "RULE_LEARNED" => "Rule Learned",
        SensorProposed = "SENSOR_PROPOSED" => "Sensor Proposed",
    }
    Swarm {
        SwarmStarted = "SWARM_STARTED" => "Swarm Started", SwarmUnitConverged = "SWARM_UNIT_CONVERGED" => "Swarm Unit Converged",
        SwarmUnitFailed = "SWARM_UNIT_FAILED" => "Swarm Unit Failed", SwarmBatonReturned = "SWARM_BATON_RETURNED" => "Swarm Baton Returned",
        SwarmCompleted = "SWARM_COMPLETED" => "Swarm Completed", SwarmDegraded = "SWARM_DEGRADED" => "Swarm Degraded",
    }
}

impl EventType {
    /// MANDATORY 8 (レジストリで ✓ 印 — 03 §6.5 M6)。
    pub const MANDATORY: &'static [EventType] = &[
        EventType::WorkflowStarted,
        EventType::WorkflowCompleted,
        EventType::WorkflowParked,
        EventType::WorkflowUnparked,
        EventType::PhaseStarted,
        EventType::PhaseCompleted,
        EventType::StageStarted,
        EventType::StageCompleted,
    ];

    /// CLI_PROTECTED 18 — 公開 audit CLI からの直接 emit を拒否する authority-bearing
    /// レシート (バイパスは `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1`)。出典: 03 §6.6 L815-819。
    pub const CLI_PROTECTED: &'static [EventType] = &[
        EventType::HumanTurn,
        EventType::GateApproved,
        EventType::GateRejected,
        EventType::QuestionAnswered,
        EventType::AutonomyModeSet,
        EventType::ReviewRequested,
        EventType::ReviewCompleted,
        EventType::PipelineLinkCompleted,
        EventType::ArtifactReused,
        EventType::SwarmStarted,
        EventType::SwarmUnitConverged,
        EventType::UnitStarted,
        EventType::UnitPaused,
        EventType::UnitResumed,
        EventType::UnitCompleted,
        EventType::DocumentIndexed,
        EventType::DocumentUpdated,
        EventType::DocumentRemoved,
    ];

    /// レジストリで `✓` が付いた MANDATORY 8 に属するか (発行が任意ではないイベント)。
    #[must_use]
    pub fn is_mandatory(self) -> bool {
        Self::MANDATORY.contains(&self)
    }

    /// CLI_PROTECTED 18 に属するか。真なら公開 audit CLI からの直接 emit は拒否され、
    /// 発行権限は所有ツール／フックに限られる。
    #[must_use]
    pub fn is_cli_protected(self) -> bool {
        Self::CLI_PROTECTED.contains(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
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
            ("WorkflowLifecycle", 4),
            ("PhaseLifecycle", 4),
            ("StageLifecycle", 6),
            ("Session", 5),
            ("Initialization", 3),
            ("Navigation", 7),
            ("Interaction", 8),
            ("UnitLifecycle", 4),
            ("Artifact", 3),
            ("Subagent", 1),
            ("ReviewerEnforcement", 2),
            ("PlanApproval", 1),
            ("Documents", 3),
            ("Utility", 1),
            ("ErrorRecovery", 2),
            ("ConstructionBolt", 4),
            ("Worktree", 7),
            ("Practices", 4),
            ("MergeDispatch", 3),
            ("Sensor", 5),
            ("LearningLoop", 3),
            ("Swarm", 6),
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
    fn every_event_has_a_distinct_heading_that_is_not_its_wire_spelling() {
        // 逐語表の 3 性質 (upstream 実測)。ここが崩れると `## <heading>` 行から
        // イベント名への逆写像が壊れ、監査ブロックの読み手が別の行を拾う。
        let headings: BTreeSet<&str> = EventType::ALL.iter().map(|e| e.heading()).collect();
        assert_eq!(headings.len(), 86, "見出しは 86 個すべて相異");
        for event in EventType::ALL {
            assert!(!event.heading().is_empty(), "{}", event.as_str());
            assert_ne!(
                event.heading(),
                event.as_str(),
                "見出しがワイヤ綴りと同一のものは upstream に 1 件も無い"
            );
        }
    }

    #[test]
    fn the_irregular_headings_are_the_ones_a_mechanical_conversion_would_miss() {
        // ワイヤ綴りからの機械変換で必ず外す箇所 (research golden-3c3146cf-audit §1 の
        // 「語形の非一様性」)。ここを固定しておかないと、後から「規則的に直した」つもりの
        // 変更が upstream 互換を静かに壊す。
        //
        // `_COMPLETED` は名詞化する組と過去分詞のままの組に割れる。
        assert_eq!(EventType::StageCompleted.heading(), "Stage Completion");
        assert_eq!(EventType::PhaseCompleted.heading(), "Phase Completion");
        assert_eq!(
            EventType::WorkflowCompleted.heading(),
            "Workflow Completion"
        );
        assert_eq!(EventType::UnitCompleted.heading(), "Unit Completed");
        assert_eq!(EventType::ReviewCompleted.heading(), "Review Completed");
        // `_STARTED` も同じく割れる。
        assert_eq!(EventType::StageStarted.heading(), "Stage Start");
        assert_eq!(EventType::SessionStarted.heading(), "Session Start");
        assert_eq!(EventType::UnitStarted.heading(), "Unit Started");
        assert_eq!(EventType::BoltStarted.heading(), "Bolt Started");
        // `SESSION_*` は名詞化、`UNIT_RESUMED` は過去分詞。
        assert_eq!(EventType::SessionResumed.heading(), "Session Resume");
        assert_eq!(EventType::SessionEnded.heading(), "Session End");
        assert_eq!(EventType::UnitResumed.heading(), "Unit Resumed");
        // `_CHANGED` / `_DETECTED` / `_VERIFIED` / `_CHECKED` / `_JUMPED` / `_SKIPPED` は名詞化。
        assert_eq!(EventType::ScopeChanged.heading(), "Scope Change");
        assert_eq!(EventType::ScopeDetected.heading(), "Scope Detection");
        assert_eq!(EventType::PhaseVerified.heading(), "Phase Verification");
        assert_eq!(EventType::HealthChecked.heading(), "Health Check");
        assert_eq!(EventType::StageJumped.heading(), "Stage Jump");
        // 語幹に無い語が付く唯一の例。
        assert_eq!(EventType::Recomposed.heading(), "Plan Recomposed");
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
