//! `IntentExecutionSnapshot` — 集約の全状態 16 属性の値オブジェクト (C6 / BR5.2)。
//!
//! 集約 → [`IntentExecution::state`]、集約 ← [`IntentExecution::from_state`] の 1 往復で
//! 永続化境界を渡る。**形の検査はしない** — 検査点は `from_state` の 1 か所に集約する
//! (security-design §2)。
//!
//! スナップショットの直列化は**この写しを経由する** (オーナー裁定 2026-08-27 (A)):
//! `IntentExecution` は `#[serde(into / try_from)]` でこの型へ委ね、復号は必ず `from_state`
//! の検査点を通る。したがって直列化形式の正本はこの 16 属性であり、復号が集約不変条件を
//! 迂回する経路は存在しない。
//!
//! 楽観 version は**この写しに載らない** (ADR-010 / B7)。本家 v3 で版数の正本が
//! `SnapshotEnvelope::version()` (スナップショット行の列) になり、payload 列は純粋な
//! ドメイン内容だけを持つようになったためである — 旧 17 属性目の `version` はここから消えた。
//!
//! [`IntentExecution::state`]: super::intent_execution::IntentExecution::state
//! [`IntentExecution::from_state`]: super::intent_execution::IntentExecution::from_state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::autonomy_mode::AutonomyMode;
use super::intent_id::IntentId;
use super::stage_entry::StageEntry;
use super::status::Status;
use crate::workflow_definition::{DefinitionRevision, PlanAction, WorkflowDefinitionId};
use crate::workspace::CheckboxState;

/// ある `seq_nr` 時点の集約の全状態 — **クレート内私有の memento**。
///
/// 状態を担うのは集約 [`IntentExecution`] であり、この型は「State」ではなくその直列化形（memento）で
/// ある（オーナー裁定 2026-08-29）。クレート外へは出さない — 出口は集約の `Serialize` /
/// `Deserialize` だけで、`IntentExecution` が `into` / `try_from` でここへ委ねる。属性の綴りと並びを
/// 変えると、既に書かれた行を読めなくなる。
///
/// [`IntentExecution`]: super::intent_execution::IntentExecution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IntentExecutionSnapshot {
    pub(crate) id: IntentId,
    pub(crate) definition_id: WorkflowDefinitionId,
    pub(crate) definition_revision: DefinitionRevision,
    pub(crate) stages: Vec<StageEntry>,
    pub(crate) plan: Vec<PlanAction>,
    pub(crate) overlay: Vec<PlanAction>,
    pub(crate) conditional: Vec<bool>,
    pub(crate) checkbox: Vec<CheckboxState>,
    pub(crate) cursor: usize,
    pub(crate) status: Status,
    pub(crate) parked_at: Option<usize>,
    pub(crate) autonomy: AutonomyMode,
    pub(crate) approved: Vec<bool>,
    pub(crate) revision_count: Vec<u32>,
    pub(crate) seq_nr: usize,
    pub(crate) last_updated_at: DateTime<Utc>,
}

/// [`IntentExecutionSnapshot`] のビルダー。
///
/// 16 属性を 1 つの関数引数列で受け取るのは可読でもリント可能でもないため、`StageNodeBuilder`
/// と同じ house style で組み立てる。既定値は解決済み計画から導ける birth 時の状態
/// (`plan` / `conditional` は `stages` の写し、`overlay` は `plan` の写し、`checkbox` は先頭のみ
/// in-progress、`approved` は全 false、`revision_count` は全 0、`cursor` = 0、`status` = running、
/// `parked_at` = なし、`autonomy` = gated、`seq_nr` = 1、`last_updated_at` = Unix epoch)。
/// `last_updated_at` の既定が epoch なのは、birth 時の発生時刻を知っているのは呼出側
/// (`Started` を作った側) だけだからである。
///
/// 可視性は写しと同じくクレート内私有であり、さらに `#[cfg(test)]` で絞ってある — 本番経路の
/// birth は `IntentExecution::start` が集約を直に起こすので、任意の状態から写しを組む必要があるのは
/// テストだけだからである (house style: `canon_json::value::arbitrary` と同じ絞り方)。
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct IntentExecutionSnapshotBuilder {
    state: IntentExecutionSnapshot,
}

#[cfg(test)]
impl IntentExecutionSnapshotBuilder {
    /// 識別子 3 種と解決済み計画から、birth 時の既定値でビルダーを起こす。
    #[must_use]
    pub(crate) fn new(
        id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        stages: Vec<StageEntry>,
    ) -> IntentExecutionSnapshotBuilder {
        let plan: Vec<PlanAction> = stages.iter().map(StageEntry::plan_action).collect();
        let conditional: Vec<bool> = stages.iter().map(StageEntry::is_conditional).collect();
        let mut checkbox = vec![CheckboxState::Pending; stages.len()];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        let approved = vec![false; stages.len()];
        let revision_count = vec![0; stages.len()];
        IntentExecutionSnapshotBuilder {
            state: IntentExecutionSnapshot {
                id,
                definition_id,
                definition_revision,
                overlay: plan.clone(),
                plan,
                conditional,
                checkbox,
                stages,
                cursor: 0,
                status: Status::Running,
                parked_at: None,
                autonomy: AutonomyMode::Gated,
                approved,
                revision_count,
                seq_nr: 1,
                last_updated_at: DateTime::UNIX_EPOCH,
            },
        }
    }

    /// 静的グリッド由来の計画を置き換える。
    #[must_use]
    pub(crate) fn plan(mut self, plan: Vec<PlanAction>) -> IntentExecutionSnapshotBuilder {
        self.state.plan = plan;
        self
    }

    /// 実効プランの源を置き換える。
    #[must_use]
    pub(crate) fn overlay(mut self, overlay: Vec<PlanAction>) -> IntentExecutionSnapshotBuilder {
        self.state.overlay = overlay;
        self
    }

    /// 適用可否の列を置き換える。
    #[must_use]
    pub(crate) fn conditional(mut self, conditional: Vec<bool>) -> IntentExecutionSnapshotBuilder {
        self.state.conditional = conditional;
        self
    }

    /// checkbox 列を置き換える。
    #[must_use]
    pub(crate) fn checkbox(
        mut self,
        checkbox: Vec<CheckboxState>,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.checkbox = checkbox;
        self
    }

    /// カーソル位置を置き換える。
    #[must_use]
    pub(crate) const fn cursor(mut self, cursor: usize) -> IntentExecutionSnapshotBuilder {
        self.state.cursor = cursor;
        self
    }

    /// ワークフロー全体の状態を置き換える。
    #[must_use]
    pub(crate) const fn status(mut self, status: Status) -> IntentExecutionSnapshotBuilder {
        self.state.status = status;
        self
    }

    /// park マーカーの位置を置き換える。
    #[must_use]
    pub(crate) const fn parked_at(
        mut self,
        parked_at: Option<usize>,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.parked_at = parked_at;
        self
    }

    /// 自律モードを置き換える。
    #[must_use]
    pub(crate) const fn autonomy(
        mut self,
        autonomy: AutonomyMode,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.autonomy = autonomy;
        self
    }

    /// ゲート承認履歴を置き換える。
    #[must_use]
    pub(crate) fn approved(mut self, approved: Vec<bool>) -> IntentExecutionSnapshotBuilder {
        self.state.approved = approved;
        self
    }

    /// 差し戻し回数の列を置き換える。
    #[must_use]
    pub(crate) fn revision_count(
        mut self,
        revision_count: Vec<u32>,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.revision_count = revision_count;
        self
    }

    /// 順序番号を置き換える。
    #[must_use]
    pub(crate) const fn seq_nr(mut self, seq_nr: usize) -> IntentExecutionSnapshotBuilder {
        self.state.seq_nr = seq_nr;
        self
    }

    /// 最終更新時刻を置き換える。
    #[must_use]
    pub(crate) const fn last_updated_at(
        mut self,
        last_updated_at: DateTime<Utc>,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.last_updated_at = last_updated_at;
        self
    }

    /// 状態の写し (memento) を取り出す (検証はしない)。
    ///
    /// 検査点は `IntentExecution::from_state` の 1 か所に集約したままにする (security-design §2) —
    /// 呼出側はこの写しを `IntentExecution::from_state` / `TryFrom` に渡して集約を得る。
    #[must_use]
    pub(crate) fn build(self) -> IntentExecutionSnapshot {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        AutonomyMode, IntentExecution, IntentId, StageDisplay, StageEntry, StageIndex, StateError,
        Status,
    };
    use crate::workflow_definition::StageNumber;
    use crate::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };
    use crate::workspace::CheckboxState;

    /// テストの表示属性 (投影は見ないので番号・表題・担当は固定でよい)。
    fn display(number: &str) -> StageDisplay {
        StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
    }

    fn entries() -> Vec<StageEntry> {
        vec![
            StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                display("0.1"),
            ),
            StageEntry::new(
                StageSlug::parse("intent-capture").unwrap(),
                PhaseId::Ideation,
                PlanAction::Execute,
                false,
                display("1.1"),
            ),
        ]
    }

    fn builder() -> IntentExecutionSnapshotBuilder {
        IntentExecutionSnapshotBuilder::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            entries(),
        )
    }

    /// 組んだ写しを検査点 (`IntentExecution::from_state`) に通して集約を得る。
    ///
    /// 観測は集約の面で行う — memento はクレート内私有であり、テストも属性を直に読まない。
    fn built(builder: IntentExecutionSnapshotBuilder) -> Result<IntentExecution, StateError> {
        IntentExecution::from_state(builder.build())
    }

    /// 索引から `StageIndex` を作る (集約だけが範囲を知っている)。
    fn at(intent: &IntentExecution, index: usize) -> StageIndex {
        intent.stage_index(index).unwrap()
    }

    #[test]
    fn the_builder_defaults_derive_the_birth_state_from_the_stage_entries() {
        let intent = built(builder()).unwrap();
        assert_eq!(intent.stages(), entries().as_slice());
        assert_eq!(
            intent.effective_plan(at(&intent, 0)),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            intent.effective_plan(at(&intent, 1)),
            Some(PlanAction::Execute)
        );
        assert!(intent.stages().iter().all(|entry| !entry.is_conditional()));
        assert_eq!(
            intent.checkbox(at(&intent, 0)),
            Some(CheckboxState::InProgress)
        );
        assert_eq!(
            intent.checkbox(at(&intent, 1)),
            Some(CheckboxState::Pending)
        );
        assert_eq!(intent.approved(at(&intent, 0)), Some(false));
        assert_eq!(intent.approved(at(&intent, 1)), Some(false));
        assert_eq!(intent.revision_count(at(&intent, 0)), Some(0));
        assert_eq!(intent.revision_count(at(&intent, 1)), Some(0));
        assert_eq!(intent.cursor(), StageIndex::new(0));
        assert_eq!(intent.status(), Status::Running);
        assert_eq!(intent.parked_at(), None);
        assert_eq!(intent.autonomy(), AutonomyMode::Gated);
        assert_eq!(intent.seq_nr(), 1);
        assert_eq!(*intent.last_updated_at(), DateTime::UNIX_EPOCH);
    }

    #[test]
    fn the_identity_attributes_are_carried_verbatim() {
        let intent = built(builder()).unwrap();
        assert_eq!(intent.id().as_str(), "01a02785-1bd8-76eb-aeea-5aa303ebd5b6");
        assert_eq!(intent.definition_id().as_str(), "claude");
        assert_eq!(
            intent.definition_revision().as_str(),
            format!("sha256:{}", "0".repeat(64))
        );
    }

    #[test]
    fn every_mutable_attribute_can_be_overridden() {
        let intent = built(
            builder()
                .overlay(vec![PlanAction::Execute, PlanAction::Skip])
                .checkbox(vec![CheckboxState::Completed, CheckboxState::Pending])
                .cursor(1)
                .status(Status::Completed)
                .parked_at(Some(1))
                .autonomy(AutonomyMode::Autonomous)
                .approved(vec![false, true])
                .revision_count(vec![0, 3])
                .seq_nr(9)
                .last_updated_at(DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5)),
        )
        .unwrap();
        assert_eq!(
            intent.effective_plan(at(&intent, 0)),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            intent.effective_plan(at(&intent, 1)),
            Some(PlanAction::Skip)
        );
        assert_eq!(
            intent.checkbox(at(&intent, 0)),
            Some(CheckboxState::Completed)
        );
        assert_eq!(
            intent.checkbox(at(&intent, 1)),
            Some(CheckboxState::Pending)
        );
        assert_eq!(intent.cursor(), StageIndex::new(1));
        assert_eq!(intent.status(), Status::Completed);
        assert_eq!(intent.parked_at(), Some(StageIndex::new(1)));
        assert_eq!(intent.autonomy(), AutonomyMode::Autonomous);
        assert_eq!(intent.approved(at(&intent, 1)), Some(true));
        assert_eq!(intent.revision_count(at(&intent, 1)), Some(3));
        assert_eq!(intent.seq_nr(), 9);
        assert_eq!(
            *intent.last_updated_at(),
            DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5)
        );
    }

    #[test]
    fn a_static_plan_or_conditional_list_that_disagrees_with_the_stages_is_refused() {
        // C6 の行としては独立した列だが、集約は解決済み計画との整合を検査する
        // (検査点は `IntentExecution::from_state` の 1 か所 — security-design §2)。
        let plan = built(builder().plan(vec![PlanAction::Skip, PlanAction::Execute])).unwrap_err();
        assert_eq!(
            plan,
            StateError::InvariantViolation("plan disagrees with stages at 0".to_string())
        );
        let conditional = built(builder().conditional(vec![true, false])).unwrap_err();
        assert_eq!(
            conditional,
            StateError::InvariantViolation("conditional disagrees with stages at 0".to_string())
        );
    }

    #[test]
    fn intents_built_from_the_same_attributes_compare_equal() {
        assert_eq!(built(builder()).unwrap(), built(builder()).unwrap());
        assert_ne!(
            built(builder()).unwrap(),
            built(builder().seq_nr(2)).unwrap()
        );
    }

    #[test]
    fn an_empty_stage_list_is_refused_by_the_aggregate() {
        // ビルダー自身は形を検証しない — 検査点は `IntentExecution::from_state` の 1 か所である。
        let err = built(IntentExecutionSnapshotBuilder::new(
            IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            Vec::new(),
        ))
        .unwrap_err();
        assert_eq!(
            err,
            StateError::InvariantViolation("stage count is zero".to_string())
        );
    }
}
