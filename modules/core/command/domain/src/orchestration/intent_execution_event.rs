//! `IntentExecutionEvent` — 29 変種のドメインイベント (C5、entities.md)。
//!
//! 変種はコマンドと 1:1 (BR1.1 / BR2.4)。ステージ参照はすべて `StageSlug` で、投影側 (U4) が
//! 索引表を要さない自己記述形になっている。イベントは構築後 immutable で、材料はアクセサで
//! 公開する。
//!
//! # 輸送のメタデータは載せない (ADR-010 / B7)
//!
//! 本家 event-store-adapter-rs v3.0.0 は `Event` trait を廃し、識別子・順序番号・発生時刻・
//! 型判別子を [`EventEnvelope`] が運ぶようになった。したがってドメインイベントは
//! **純粋なドメイン内容だけ**を持つ (本家の語で payload)。かつて自前で持っていた封筒
//! (`id` / `schema_version` / `occurred_at`) と、その識別子型は削除し、封筒を
//! 組むのはアダプタ層 (Repository) の責務にした — 「Payload」は輸送の語であってドメインの語では
//! ないので、この enum 自身がドメインイベントの正体である (ubiquitous-language.md)。
//!
//! **直列化の記述は持たない** (改訂 9 / `coding-rules/domain-persistence-neutrality.md`)。
//! 行のバイトを決めるのは書く側 (command interface-adapter) と読む側 (RMU) の DTO であり、
//! この enum が持つのはドメインの語彙だけである。
//!
//! [`EventEnvelope`]: https://docs.rs/event-store-adapter-rs/3.0.0/event_store_adapter_rs/event_envelope/struct.EventEnvelope.html

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — 利便再エクスポートではない。
// coding-rules/module-visibility.md)。
use super::intent_execution_event_id::IntentExecutionEventId;
use super::intent_execution_id::IntentExecutionId;

mod single_stage_run_started;
pub use single_stage_run_started::SingleStageRunStarted;
mod pipeline_link_completed;
pub use pipeline_link_completed::PipelineLinkCompleted;
mod answer_recorded;
mod plan_answer_logged;
pub use plan_answer_logged::PlanAnswerLogged;
mod directive_issued;
pub use directive_issued::DirectiveIssued;
mod directive_context_invalidated;
pub use directive_context_invalidated::DirectiveContextInvalidated;
mod autonomy_mode_set;
mod command_failed;
mod health_checked;
pub use health_checked::HealthChecked;
mod memory_journals_observed;
pub use memory_journals_observed::MemoryJournalsObserved;
mod learnings_captured;
pub use learnings_captured::LearningsCaptured;
mod task_synchronized;
pub use task_synchronized::TaskSynchronized;
mod decision_recorded;
pub use answer_recorded::AnswerRecorded;
pub use command_failed::CommandFailed;
mod prompt_observed;
pub use prompt_observed::PromptObserved;
mod gate_approved;
mod gate_opened;
mod gate_rejected;
mod jumped;
mod parked;
mod practices_affirmed;
mod recomposed;
mod reported;
pub use decision_recorded::DecisionRecorded;
mod review_completed;
mod review_requested;
mod single_stage_run_committed;
mod skeleton_stance_recorded;
mod stage_revised;
mod stage_skipped;
mod started;
mod unparked;
pub use reported::Reported;

pub use autonomy_mode_set::AutonomyModeSet;
pub use gate_approved::GateApproved;
pub use gate_opened::GateOpened;
pub use gate_rejected::GateRejected;
pub use jumped::Jumped;
pub use parked::Parked;
pub use practices_affirmed::PracticesAffirmed;
pub use recomposed::Recomposed;
pub use review_completed::ReviewCompleted;
pub use review_requested::ReviewRequested;
pub use single_stage_run_committed::SingleStageRunCommitted;
pub use skeleton_stance_recorded::SkeletonStanceRecorded;
pub use stage_revised::StageRevised;
pub use stage_skipped::StageSkipped;
pub use started::Started;
pub use unparked::Unparked;

/// 29 変種のドメインイベント (C5)。
///
/// `#[non_exhaustive]` は**付けない** — 変種の追加は C5 の改訂を伴う設計事項であり、消費側の
/// 網羅 match が落ちること自体が検出手段である (NFR1.3)。
///
/// # イベントはエンティティ — 全変種が `id` と `aggregate_id` を持つ
///
/// ドメインイベントはエンティティの一種なので、変種ごとに自前の識別子
/// [`IntentExecutionEventId`] を持ち、どの集約の事実かは別フィールド `aggregate_id` が運ぶ
/// (オーナー裁定 2026-09-02、`coding-rules/domain-object-kinds.md` /
/// `coding-rules/aggregate-commands.md`)。採番は集約のコマンド内 (`generate`) であり、通番
/// `seq_nr` と発生時刻 `occurred_at` は従来どおり封筒が運ぶ (ADR-010 / B7)。
///
/// `Unparked` は C5 が `payload: {}` とする材料なしの事実だが、それでも識別子は持つので
/// 単位変種ではなく [`Unparked`] 構造体を張る。
///
/// [`IntentExecutionEventId`]: super::intent_execution_event_id::IntentExecutionEventId
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionEvent {
    /// 単独pipeline試行の開始。
    SingleStageRunStarted(SingleStageRunStarted),
    /// 宣言されたpipeline linkの完了受領。
    PipelineLinkCompleted(PipelineLinkCompleted),
    /// 保護された計画回答を元の実行へ監査記録した。
    PlanAnswerLogged(Box<PlanAnswerLogged>),
    /// ハーネスへ指示を発行した事実。
    DirectiveIssued(DirectiveIssued),
    /// 発行済み文脈の失効。公開監査はPreCompactの観測が担う。
    DirectiveContextInvalidated(DirectiveContextInvalidated),
    /// 回答を受理した事実と結果。
    AnswerRecorded(AnswerRecorded),
    /// フックが受け取った応答。
    PromptObserved(PromptObserved),
    /// 通常の質問を提示した事実。
    DecisionRecorded(DecisionRecorded),
    /// コマンドが失敗した事実。
    CommandFailed(CommandFailed),
    /// 作業の診断を実施した事実。
    HealthChecked(HealthChecked),
    /// runtime-graph の compile がステージ日誌を読んだ事実。
    ///
    /// 観測は runtime-graph の材料、判定は `MEMORY_EMPTY` の材料である。作業の進行も
    /// 承認も変えない。
    MemoryJournalsObserved(Box<MemoryJournalsObserved>),
    /// §13 の儀式が確定した学びをメモリ層へ書き写すと決めた事実。
    ///
    /// 実践行と監査行 `RULE_LEARNED` の材料である。作業の進行も承認も変えない。
    LearningsCaptured(Box<LearningsCaptured>),
    /// TaskUpdate が指す作業 stage へ現在位置を同期した事実。
    ///
    /// 他 stage の進捗は保持する。stage 開始監査もゲート承認も意味しない。
    TaskSynchronized(TaskSynchronized),
    /// 報告を受理した事実とその結果。
    Reported(Reported),
    /// 実行の開始。
    Started(Started),
    /// 承認ゲートの開放。
    GateOpened(GateOpened),
    /// 承認ゲートの通過。
    GateApproved(GateApproved),
    /// 承認ゲートでの差し戻し。
    GateRejected(GateRejected),
    /// 差し戻し後のゲート再入。
    StageRevised(StageRevised),
    /// ステージの読み飛ばし。
    StageSkipped(StageSkipped),
    /// カーソルの移動 (forward / backward / redo)。
    Jumped(Jumped),
    /// park マーカーの設置。
    Parked(Parked),
    /// park マーカーの除去 (位置は `parked_at` から復元されるので材料なし)。
    Unparked(Unparked),
    /// 実効プランの再形成 (オーバレイの反転)。
    Recomposed(Recomposed),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSet),
    /// 隔離実行 (`--single`) の疑似ワークフロー ID 付き対の記録 (**適用はフレーム空**)。
    SingleStageRunCommitted(SingleStageRunCommitted),
    /// conductor が分類した walking-skeleton stance の記録。
    SkeletonStanceRecorded(SkeletonStanceRecorded),
    /// レビュアーの差し向け (受領証の対の左半分)。
    ReviewRequested(ReviewRequested),
    /// レビュアーの判定の記録 (受領証の対の右半分)。
    ReviewCompleted(ReviewCompleted),
    /// 承認された実践がメモリ層の正本へ書き写された事実 (practices-discovery の受領証)。
    PracticesAffirmed(PracticesAffirmed),
}

impl IntentExecutionEvent {
    /// 次の実効ステージへの前進を生じた、完了または読み飛ばしの対象。
    pub(super) const fn advancing_stage(&self) -> Option<&crate::workflow_definition::StageSlug> {
        match self {
            Self::GateApproved(event) => Some(event.stage()),
            Self::StageSkipped(event) => Some(event.stage()),
            Self::Reported(event) => match event.result() {
                super::ReportResult::Committed {
                    stage,
                    transition:
                        super::ReportTransition::GateApproved { .. }
                        | super::ReportTransition::StageSkipped { .. },
                    ..
                } => Some(stage),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) const fn affects_progress(&self) -> bool {
        match self {
            Self::Reported(event) => matches!(
                event.result(),
                crate::orchestration::ReportResult::Committed { .. }
            ),
            Self::SingleStageRunStarted(_)
            | Self::PipelineLinkCompleted(_)
            | Self::PlanAnswerLogged(_)
            | Self::DirectiveIssued(_)
            | Self::DirectiveContextInvalidated(_)
            | Self::DecisionRecorded(_)
            | Self::CommandFailed(_)
            | Self::HealthChecked(_)
            | Self::MemoryJournalsObserved(_)
            | Self::LearningsCaptured(_)
            | Self::PromptObserved(_)
            | Self::AnswerRecorded(_)
            | Self::ReviewRequested(_)
            | Self::ReviewCompleted(_)
            | Self::SingleStageRunCommitted(_) => false,
            Self::Started(_)
            | Self::GateOpened(_)
            | Self::GateApproved(_)
            | Self::GateRejected(_)
            | Self::StageRevised(_)
            | Self::StageSkipped(_)
            | Self::Jumped(_)
            | Self::Parked(_)
            | Self::Unparked(_)
            | Self::Recomposed(_)
            | Self::AutonomyModeSet(_)
            | Self::SkeletonStanceRecorded(_)
            | Self::TaskSynchronized(_)
            | Self::PracticesAffirmed(_) => true,
        }
    }

    /// このイベント自身の識別子 (全変種が持つ — イベントはエンティティ)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        match self {
            IntentExecutionEvent::DecisionRecorded(payload) => payload.id(),
            IntentExecutionEvent::CommandFailed(payload) => payload.id(),
            IntentExecutionEvent::HealthChecked(payload) => payload.id(),
            IntentExecutionEvent::MemoryJournalsObserved(payload) => payload.id(),
            IntentExecutionEvent::LearningsCaptured(payload) => payload.id(),
            IntentExecutionEvent::TaskSynchronized(payload) => payload.id(),
            IntentExecutionEvent::PromptObserved(payload) => payload.id(),
            IntentExecutionEvent::SingleStageRunStarted(payload) => payload.id(),
            IntentExecutionEvent::PipelineLinkCompleted(payload) => payload.id(),
            IntentExecutionEvent::PlanAnswerLogged(payload) => payload.id(),
            IntentExecutionEvent::AnswerRecorded(payload) => payload.id(),
            IntentExecutionEvent::DirectiveIssued(payload) => payload.id(),
            IntentExecutionEvent::DirectiveContextInvalidated(payload) => payload.id(),
            IntentExecutionEvent::Reported(payload) => payload.id(),
            IntentExecutionEvent::Started(payload) => payload.id(),
            IntentExecutionEvent::GateOpened(payload) => payload.id(),
            IntentExecutionEvent::GateApproved(payload) => payload.id(),
            IntentExecutionEvent::GateRejected(payload) => payload.id(),
            IntentExecutionEvent::StageRevised(payload) => payload.id(),
            IntentExecutionEvent::StageSkipped(payload) => payload.id(),
            IntentExecutionEvent::Jumped(payload) => payload.id(),
            IntentExecutionEvent::Parked(payload) => payload.id(),
            IntentExecutionEvent::Unparked(payload) => payload.id(),
            IntentExecutionEvent::Recomposed(payload) => payload.id(),
            IntentExecutionEvent::AutonomyModeSet(payload) => payload.id(),
            IntentExecutionEvent::SingleStageRunCommitted(payload) => payload.id(),
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => payload.id(),
            IntentExecutionEvent::ReviewRequested(payload) => payload.id(),
            IntentExecutionEvent::ReviewCompleted(payload) => payload.id(),
            IntentExecutionEvent::PracticesAffirmed(payload) => payload.id(),
        }
    }

    /// **どの集約の事実か** — 全変種が運ぶ実行の識別子。
    ///
    /// 復号境界 (Repository の再生・RMU の `decode_entry`) はこれと行の `aid` を照合する。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        match self {
            IntentExecutionEvent::DecisionRecorded(payload) => payload.aggregate_id(),
            IntentExecutionEvent::CommandFailed(payload) => payload.aggregate_id(),
            IntentExecutionEvent::HealthChecked(payload) => payload.aggregate_id(),
            IntentExecutionEvent::MemoryJournalsObserved(payload) => payload.aggregate_id(),
            IntentExecutionEvent::LearningsCaptured(payload) => payload.aggregate_id(),
            IntentExecutionEvent::TaskSynchronized(payload) => payload.aggregate_id(),
            IntentExecutionEvent::PromptObserved(payload) => payload.aggregate_id(),
            IntentExecutionEvent::SingleStageRunStarted(payload) => payload.aggregate_id(),
            IntentExecutionEvent::PipelineLinkCompleted(payload) => payload.aggregate_id(),
            IntentExecutionEvent::PlanAnswerLogged(payload) => payload.aggregate_id(),
            IntentExecutionEvent::AnswerRecorded(payload) => payload.aggregate_id(),
            IntentExecutionEvent::DirectiveIssued(payload) => payload.aggregate_id(),
            IntentExecutionEvent::DirectiveContextInvalidated(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Reported(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Started(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateOpened(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateApproved(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateRejected(payload) => payload.aggregate_id(),
            IntentExecutionEvent::StageRevised(payload) => payload.aggregate_id(),
            IntentExecutionEvent::StageSkipped(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Jumped(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Parked(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Unparked(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Recomposed(payload) => payload.aggregate_id(),
            IntentExecutionEvent::AutonomyModeSet(payload) => payload.aggregate_id(),
            IntentExecutionEvent::SingleStageRunCommitted(payload) => payload.aggregate_id(),
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => payload.aggregate_id(),
            IntentExecutionEvent::ReviewRequested(payload) => payload.aggregate_id(),
            IntentExecutionEvent::ReviewCompleted(payload) => payload.aggregate_id(),
            IntentExecutionEvent::PracticesAffirmed(payload) => payload.aggregate_id(),
        }
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use std::collections::HashSet;

    use super::*;
    use crate::orchestration::{
        ArtifactPaths, AutonomyMode, ReviewVerdict, SkeletonStance, StageDisplay, StageEntries,
        StageEntry, StageIndex, StageSlugSet,
    };
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};
    use crate::workspace::{PromotedSection, PromotedSections, RuleLines};

    use super::super::intent_id::IntentId;

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    /// 決め打ちのイベント識別子 (綴りを固定したいテスト用)。
    fn evid() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap()
    }

    fn agg() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
    }

    /// genesis の材料を束ねたペイロード (計画は 1 ステージの最小形)。
    fn started() -> Started {
        Started::new(
            evid(),
            agg(),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            StageEntries::new(vec![StageEntry::new(
                slug("state-init"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(
                    StageNumber::parse("0.1").unwrap(),
                    "State Init",
                    "orchestrator",
                )
                .unwrap(),
            )])
            .unwrap(),
        )
    }

    fn plan_answer_sample() -> PlanAnswerLogged {
        use crate::orchestration::*;
        let authority = CodeGenerationAuthority::new(
            &PlanTarget::stage_level(),
            &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            format!("sha256:{}", "a".repeat(64)),
            "unstarted#0".into(),
            "b".repeat(64),
            1,
        )
        .unwrap();
        let evidence = PlanApprovalEvidence::new(
            authority,
            format!("sha256:{}", "c".repeat(64)),
            "questions.md".into(),
            "d".repeat(64),
            "e".repeat(64),
        )
        .unwrap();
        PlanAnswerLogged::new(
            evid(),
            agg(),
            PlanApprovalOperationId::generate(),
            PlanAnswerInput::new(
                PlanApprovalOrigin::new(crate::workspace::SpaceName::default(), agg()),
                "code-generation".into(),
                PlanDecisionEvidence::new(evidence, PlanSession::new("session".into()).unwrap()),
                PlanChoice::ApprovePlan,
                Some("b".repeat(64)),
            ),
        )
    }

    /// 全変種を 1 つずつ (同じ id / aggregate_id で組む)。
    fn every_variant() -> Vec<IntentExecutionEvent> {
        vec![
            IntentExecutionEvent::SingleStageRunStarted(SingleStageRunStarted::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::PlanAnswerLogged(Box::new(plan_answer_sample())),
            IntentExecutionEvent::DirectiveContextInvalidated(DirectiveContextInvalidated::new(
                evid(),
                agg(),
                crate::orchestration::ActiveDirective::new(
                    1,
                    IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
                    crate::orchestration::DirectivePublication::new(
                        "a".repeat(64),
                        "b".repeat(64),
                        crate::orchestration::PublishedDirective::Error {
                            stage: slug("intent-capture"),
                        },
                    ),
                    "b".repeat(64),
                    "sessionless:1111111111111111".to_string(),
                    0,
                    0,
                    1,
                ),
            )),
            IntentExecutionEvent::DirectiveIssued(DirectiveIssued::new(
                evid(),
                agg(),
                crate::orchestration::ActiveDirective::new(
                    1,
                    IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
                    crate::orchestration::DirectivePublication::new(
                        "a".repeat(64),
                        "b".repeat(64),
                        crate::orchestration::PublishedDirective::RunStage {
                            stage: slug("intent-capture"),
                            unit: None,
                        },
                    ),
                    "b".repeat(64),
                    "sessionless:1111111111111111".to_string(),
                    0,
                    0,
                    1,
                ),
            )),
            IntentExecutionEvent::AnswerRecorded(AnswerRecorded::new(
                evid(),
                agg(),
                crate::orchestration::AnswerId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6")
                    .unwrap(),
                "requirements-analysis",
                "A",
                crate::orchestration::AnswerDisposition::Recorded,
            )),
            IntentExecutionEvent::PromptObserved(PromptObserved::new(
                evid(),
                agg(),
                "session",
                "1",
                false,
            )),
            IntentExecutionEvent::PipelineLinkCompleted(PipelineLinkCompleted::new(
                evid(),
                agg(),
                crate::orchestration::PipelineReceipt::new(
                    "reverse-engineering".into(),
                    "aidlc-architect-agent".into(),
                    None,
                    false,
                    2,
                    2,
                    None,
                )
                .unwrap(),
            )),
            IntentExecutionEvent::HealthChecked(HealthChecked::new(
                evid(),
                agg(),
                crate::orchestration::HealthCheckResult::new(4, 1),
            )),
            IntentExecutionEvent::CommandFailed(CommandFailed::new(
                evid(),
                agg(),
                crate::orchestration::CommandFailure::new(
                    "aidlc-log".into(),
                    "aidlc-log review".into(),
                    "Missing --stage <slug>".into(),
                ),
            )),
            IntentExecutionEvent::DecisionRecorded(DecisionRecorded::new(
                evid(),
                agg(),
                crate::orchestration::DecisionPrompt::new("requirements-analysis", "質問"),
            )),
            IntentExecutionEvent::Reported(
                Reported::new(
                    evid(),
                    agg(),
                    crate::orchestration::ReportId::generate(),
                    crate::orchestration::ReportResult::NoOp {
                        scope: "bugfix".into(),
                        no_op: crate::orchestration::ReportNoOp::AlreadyAwaiting {
                            stage: slug("requirements-analysis"),
                        },
                    },
                    None,
                    None,
                )
                .unwrap(),
            ),
            IntentExecutionEvent::Started(started()),
            IntentExecutionEvent::GateOpened(GateOpened::new(
                evid(),
                agg(),
                slug("intent-capture"),
                ArtifactPaths::new(vec!["intent.md".to_string()]),
            )),
            IntentExecutionEvent::GateApproved(GateApproved::new(
                evid(),
                agg(),
                slug("intent-capture"),
                Some("looks good".to_string()),
            )),
            IntentExecutionEvent::GateRejected(GateRejected::new(
                evid(),
                agg(),
                slug("intent-capture"),
                None,
            )),
            IntentExecutionEvent::StageRevised(StageRevised::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                evid(),
                agg(),
                slug("market-research"),
                "out of scope".to_string(),
            )),
            IntentExecutionEvent::Jumped(Jumped::new(
                evid(),
                agg(),
                slug("state-init"),
                crate::orchestration::JumpDirection::Redo,
                None,
            )),
            IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture"))),
            IntentExecutionEvent::Unparked(Unparked::new(evid(), agg())),
            IntentExecutionEvent::Recomposed(Recomposed::new(
                evid(),
                agg(),
                StageSlugSet::new([slug("market-research")]),
                StageSlugSet::empty(),
            )),
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                evid(),
                agg(),
                AutonomyMode::Autonomous,
            )),
            IntentExecutionEvent::SingleStageRunCommitted(SingleStageRunCommitted::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::SkeletonStanceRecorded(SkeletonStanceRecorded::new(
                evid(),
                agg(),
                SkeletonStance::On,
            )),
            IntentExecutionEvent::ReviewRequested(ReviewRequested::new(
                evid(),
                agg(),
                slug("intent-capture"),
                "aidlc-product-lead-agent",
                1,
                false,
                crate::orchestration::review_test_fixture::binding(),
            )),
            IntentExecutionEvent::ReviewCompleted(ReviewCompleted::new(
                evid(),
                agg(),
                slug("intent-capture"),
                "aidlc-product-lead-agent",
                1,
                ReviewVerdict::Ready,
                crate::orchestration::review_test_fixture::completion(),
            )),
            IntentExecutionEvent::PracticesAffirmed(PracticesAffirmed::new(
                evid(),
                agg(),
                slug("practices-discovery"),
                "owner",
                PromotedSections::new(vec![PromotedSection::new(
                    "Way of Working",
                    "trunk-based.\n",
                )])
                .unwrap(),
                RuleLines::new(vec!["ALWAYS review. (affirmed 2026-09-05)".to_string()]),
                RuleLines::new(vec!["NEVER force-push. (affirmed 2026-09-05)".to_string()]),
            )),
            IntentExecutionEvent::TaskSynchronized(TaskSynchronized::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::LearningsCaptured(Box::new(LearningsCaptured::new(
                evid(),
                agg(),
                slug("requirements-analysis"),
                crate::orchestration::LearningProvenance::new(
                    crate::workspace::SpaceName::default(),
                    crate::workspace::IntentDirName::parse("260908-learnings").unwrap(),
                ),
                crate::orchestration::CapturedLearnings::new(vec![
                    crate::orchestration::CapturedLearning::new(
                        crate::orchestration::Learning::new(
                            crate::orchestration::LearningCandidateId::parse("c1").unwrap(),
                            crate::orchestration::LearningScope::Project,
                            crate::orchestration::PracticeHeading::corrections(),
                            "ALWAYS record the evidence",
                            crate::orchestration::LearningSource::Orchestrator,
                        ),
                        crate::orchestration::LearningDisposition::Fresh,
                    ),
                ]),
            ))),
        ]
    }

    #[test]
    fn the_started_payload_carries_the_genesis_material() {
        // genesis の材料 (実行 id・intent id・解決済み計画) を運ぶ — 誕生状態の導出に
        // `&Intent` を要さないので、実行のストリームは自ストリームだけで再生できる。
        let started = started();
        assert_eq!(
            started.aggregate_id().as_str(),
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
        );
        assert_eq!(
            started.intent_id().as_str(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(started.stages().len(), 1);
        assert_eq!(
            started
                .stages()
                .at(StageIndex::new(0))
                .map(StageEntry::slug),
            Some(&slug("state-init"))
        );
    }

    #[test]
    fn the_stage_lifecycle_payloads_carry_their_slugs_and_material() {
        let opened = GateOpened::new(
            evid(),
            agg(),
            slug("intent-capture"),
            ArtifactPaths::new(vec!["intent.md".to_string()]),
        );
        assert_eq!(opened.stage(), &slug("intent-capture"));
        assert_eq!(
            opened.artifacts(),
            &ArtifactPaths::new(vec!["intent.md".to_string()])
        );

        let approved = GateApproved::new(
            evid(),
            agg(),
            slug("intent-capture"),
            Some("looks good".to_string()),
        );
        assert_eq!(approved.stage(), &slug("intent-capture"));
        assert_eq!(approved.user_input(), Some("looks good"));

        let rejected = GateRejected::new(evid(), agg(), slug("intent-capture"), None);
        assert_eq!(rejected.stage(), &slug("intent-capture"));
        assert_eq!(rejected.feedback(), None);

        let revised = StageRevised::new(evid(), agg(), slug("intent-capture"));
        assert_eq!(revised.stage(), &slug("intent-capture"));

        let skipped = StageSkipped::new(
            evid(),
            agg(),
            slug("market-research"),
            "out of scope".to_string(),
        );
        assert_eq!(skipped.stage(), &slug("market-research"));
        assert_eq!(skipped.reason(), "out of scope");
    }

    #[test]
    fn the_control_payloads_carry_the_jump_park_and_recompose_material() {
        let jumped = Jumped::new(
            evid(),
            agg(),
            slug("state-init"),
            crate::orchestration::JumpDirection::Redo,
            None,
        );
        assert_eq!(jumped.target(), &slug("state-init"));

        let parked = Parked::new(evid(), agg(), slug("intent-capture"));
        assert_eq!(parked.stage(), &slug("intent-capture"));

        // `Unparked` は材料なしだが単位変種ではない — 識別子は持つ。
        let unparked = Unparked::new(evid(), agg());
        assert_eq!(unparked.id(), &evid());
        assert_eq!(unparked.aggregate_id(), &agg());

        let recomposed = Recomposed::new(
            evid(),
            agg(),
            StageSlugSet::new([slug("market-research")]),
            StageSlugSet::empty(),
        );
        assert_eq!(
            recomposed.skipped(),
            &StageSlugSet::new([slug("market-research")])
        );
        assert!(recomposed.added().is_empty());

        let mode = AutonomyModeSet::new(evid(), agg(), AutonomyMode::Autonomous);
        assert_eq!(mode.mode(), AutonomyMode::Autonomous);
    }

    #[test]
    fn every_variant_answers_its_own_id_and_its_aggregate_id() {
        // イベントはエンティティ — 変種によらず自前の id と「どの集約の事実か」を答える。
        for event in every_variant() {
            assert_eq!(event.id(), &evid());
            assert_eq!(event.aggregate_id(), &agg());
        }
    }

    #[test]
    fn events_compare_by_value() {
        let a = IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture")));
        let b = IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture")));
        assert_eq!(a, b);
        assert_ne!(
            a,
            IntentExecutionEvent::Unparked(Unparked::new(evid(), agg()))
        );
    }

    #[test]
    fn two_events_of_the_same_shape_are_distinguished_by_their_generated_ids() {
        // 同じ材料でも別の事実である — 識別子が違えば別のエンティティになる。
        let first = Parked::new(
            IntentExecutionEventId::generate(),
            agg(),
            slug("intent-capture"),
        );
        let second = Parked::new(
            IntentExecutionEventId::generate(),
            agg(),
            slug("intent-capture"),
        );
        assert_ne!(first.id(), second.id());
        assert_ne!(first, second);
    }

    #[test]
    fn every_variant_is_matched_exhaustively() {
        // NFR1.3 — 変種の追加は C5 の改訂を伴うので `#[non_exhaustive]` は付けない。
        // 本テストは網羅 match をコンパイル時に固定する (腕が欠けたらビルドが落ちる)。
        const fn name(payload: &IntentExecutionEvent) -> &'static str {
            match payload {
                IntentExecutionEvent::DirectiveIssued(_) => "DirectiveIssued",
                IntentExecutionEvent::DirectiveContextInvalidated(_) => {
                    "DirectiveContextInvalidated"
                }
                IntentExecutionEvent::PlanAnswerLogged(_) => "PlanAnswerLogged",
                IntentExecutionEvent::AnswerRecorded(_) => "AnswerRecorded",
                IntentExecutionEvent::PromptObserved(_) => "PromptObserved",
                IntentExecutionEvent::DecisionRecorded(_) => "DecisionRecorded",
                IntentExecutionEvent::CommandFailed(_) => "CommandFailed",
                IntentExecutionEvent::HealthChecked(_) => "HealthChecked",
                IntentExecutionEvent::MemoryJournalsObserved(_) => "MemoryJournalsObserved",
                IntentExecutionEvent::LearningsCaptured(_) => "LearningsCaptured",
                IntentExecutionEvent::SingleStageRunStarted(_) => "SingleStageRunStarted",
                IntentExecutionEvent::PipelineLinkCompleted(_) => "PipelineLinkCompleted",
                IntentExecutionEvent::Reported(_) => "Reported",
                IntentExecutionEvent::Started(_) => "Started",
                IntentExecutionEvent::GateOpened(_) => "GateOpened",
                IntentExecutionEvent::GateApproved(_) => "GateApproved",
                IntentExecutionEvent::GateRejected(_) => "GateRejected",
                IntentExecutionEvent::StageRevised(_) => "StageRevised",
                IntentExecutionEvent::StageSkipped(_) => "StageSkipped",
                IntentExecutionEvent::Jumped(_) => "Jumped",
                IntentExecutionEvent::Parked(_) => "Parked",
                IntentExecutionEvent::Unparked(_) => "Unparked",
                IntentExecutionEvent::Recomposed(_) => "Recomposed",
                IntentExecutionEvent::AutonomyModeSet(_) => "AutonomyModeSet",
                IntentExecutionEvent::SingleStageRunCommitted(_) => "SingleStageRunCommitted",
                IntentExecutionEvent::SkeletonStanceRecorded(_) => "SkeletonStanceRecorded",
                IntentExecutionEvent::ReviewRequested(_) => "ReviewRequested",
                IntentExecutionEvent::ReviewCompleted(_) => "ReviewCompleted",
                IntentExecutionEvent::PracticesAffirmed(_) => "PracticesAffirmed",
                IntentExecutionEvent::TaskSynchronized(_) => "TaskSynchronized",
            }
        }
        let expected = [
            "SingleStageRunStarted",
            "PlanAnswerLogged",
            "DirectiveContextInvalidated",
            "DirectiveIssued",
            "AnswerRecorded",
            "PromptObserved",
            "PipelineLinkCompleted",
            "HealthChecked",
            "CommandFailed",
            "DecisionRecorded",
            "Reported",
            "Started",
            "GateOpened",
            "GateApproved",
            "GateRejected",
            "StageRevised",
            "StageSkipped",
            "Jumped",
            "Parked",
            "Unparked",
            "Recomposed",
            "AutonomyModeSet",
            "SingleStageRunCommitted",
            "SkeletonStanceRecorded",
            "ReviewRequested",
            "ReviewCompleted",
            "PracticesAffirmed",
            "TaskSynchronized",
            "LearningsCaptured",
        ];
        let named: Vec<&'static str> = every_variant().iter().map(name).collect();
        assert_eq!(named, expected);
        let distinct: HashSet<&'static str> = named.iter().copied().collect();
        assert_eq!(distinct.len(), 29);
    }
}
