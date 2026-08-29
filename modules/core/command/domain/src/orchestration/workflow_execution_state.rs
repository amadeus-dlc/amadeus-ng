//! `WorkflowExecutionState` — 集約の全状態 16 属性の値オブジェクト (C6 / BR5.2)。
//!
//! 集約 → [`WorkflowExecution::state`]、集約 ← [`WorkflowExecution::from_state`] の 1 往復で
//! 永続化境界を渡る。**形の検査はしない** — 検査点は `from_state` の 1 か所に集約する
//! (security-design §2)。
//!
//! スナップショットの直列化は**この写しを経由する** (オーナー裁定 2026-08-27 (A)):
//! `WorkflowExecution` は `#[serde(into / try_from)]` でこの型へ委ね、復号は必ず `from_state`
//! の検査点を通る。したがって直列化形式の正本はこの 16 属性であり、復号が集約不変条件を
//! 迂回する経路は存在しない。
//!
//! 楽観 version は**この写しに載らない** (ADR-010 / B7)。本家 v3 で版数の正本が
//! `SnapshotEnvelope::version()` (スナップショット行の列) になり、payload 列は純粋な
//! ドメイン内容だけを持つようになったためである — 旧 17 属性目の `version` はここから消えた。
//!
//! [`WorkflowExecution::state`]: super::workflow_execution::WorkflowExecution::state
//! [`WorkflowExecution::from_state`]: super::workflow_execution::WorkflowExecution::from_state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::autonomy_mode::AutonomyMode;
use super::intent_id::IntentId;
use super::stage_entry::StageEntry;
use super::stage_index::StageIndex;
use super::status::Status;
use crate::workflow_definition::{DefinitionRevision, PlanAction, WorkflowDefinitionId};
use crate::workspace::CheckboxState;

/// ある `seq_nr` 時点の集約の全状態。
///
/// フィールドは `pub(crate)` — 同一クレート内の実装詳細共有 (集約との再水和) にだけ開き、
/// クレート外へは 16 本のアクセサでのみ公開する (field-visibility.md)。
///
/// serde は集約の直列化形式そのものである (集約が `into` / `try_from` でここへ委ねる)。
/// 属性の綴りと並びを変えると、既に書かれた行を読めなくなる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowExecutionState {
    pub(crate) intent_id: IntentId,
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

impl WorkflowExecutionState {
    /// 集約識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 参照した定義の系譜 ID。
    #[must_use]
    pub const fn definition_id(&self) -> &WorkflowDefinitionId {
        &self.definition_id
    }

    /// 参照した定義の内容版。
    #[must_use]
    pub const fn definition_revision(&self) -> &DefinitionRevision {
        &self.definition_revision
    }

    /// 文書順の全ステージ (解決済み計画)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }

    /// 静的グリッド由来の計画。
    #[must_use]
    pub fn plan(&self) -> &[PlanAction] {
        &self.plan
    }

    /// 実効プランの源 (recompose オーバレイ)。
    #[must_use]
    pub fn overlay(&self) -> &[PlanAction] {
        &self.overlay
    }

    /// ステージ著者側の適用可否 (CONDITIONAL か)。
    #[must_use]
    pub fn conditional(&self) -> &[bool] {
        &self.conditional
    }

    /// ステージごとの checkbox マーカー。
    #[must_use]
    pub fn checkbox(&self) -> &[CheckboxState] {
        &self.checkbox
    }

    /// カーソル位置。
    #[must_use]
    pub const fn cursor(&self) -> StageIndex {
        StageIndex::new(self.cursor)
    }

    /// ワークフロー全体の状態。
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// park マーカーの位置 (`None` は未 park)。
    #[must_use]
    pub fn parked_at(&self) -> Option<StageIndex> {
        self.parked_at.map(StageIndex::new)
    }

    /// 自律モード。
    #[must_use]
    pub const fn autonomy(&self) -> AutonomyMode {
        self.autonomy
    }

    /// ゲート承認履歴。
    #[must_use]
    pub fn approved(&self) -> &[bool] {
        &self.approved
    }

    /// ステージごとの差し戻し回数。
    #[must_use]
    pub fn revision_count(&self) -> &[u32] {
        &self.revision_count
    }

    /// 適用済みイベント数と一致する順序番号。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 最後に適用したイベントの発生時刻。
    #[must_use]
    pub const fn last_updated_at(&self) -> DateTime<Utc> {
        self.last_updated_at
    }
}

/// [`WorkflowExecutionState`] のビルダー。
///
/// 16 属性を 1 つの関数引数列で受け取るのは可読でもリント可能でもないため、`StageNodeBuilder`
/// と同じ house style で組み立てる。既定値は解決済み計画から導ける birth 時の状態
/// (`plan` / `conditional` は `stages` の写し、`overlay` は `plan` の写し、`checkbox` は先頭のみ
/// in-progress、`approved` は全 false、`revision_count` は全 0、`cursor` = 0、`status` = running、
/// `parked_at` = なし、`autonomy` = gated、`seq_nr` = 1、`last_updated_at` = Unix epoch)。
/// `last_updated_at` の既定が epoch なのは、birth 時の発生時刻を知っているのは呼出側
/// (`Started` を作った側) だけだからである。
#[derive(Debug, Clone)]
pub struct WorkflowExecutionStateBuilder {
    state: WorkflowExecutionState,
}

impl WorkflowExecutionStateBuilder {
    /// 識別子 3 種と解決済み計画から、birth 時の既定値でビルダーを起こす。
    #[must_use]
    pub fn new(
        intent_id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        stages: Vec<StageEntry>,
    ) -> WorkflowExecutionStateBuilder {
        let plan: Vec<PlanAction> = stages.iter().map(StageEntry::plan_action).collect();
        let conditional: Vec<bool> = stages.iter().map(StageEntry::is_conditional).collect();
        let mut checkbox = vec![CheckboxState::Pending; stages.len()];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        let approved = vec![false; stages.len()];
        let revision_count = vec![0; stages.len()];
        WorkflowExecutionStateBuilder {
            state: WorkflowExecutionState {
                intent_id,
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
    pub fn plan(mut self, plan: Vec<PlanAction>) -> WorkflowExecutionStateBuilder {
        self.state.plan = plan;
        self
    }

    /// 実効プランの源を置き換える。
    #[must_use]
    pub fn overlay(mut self, overlay: Vec<PlanAction>) -> WorkflowExecutionStateBuilder {
        self.state.overlay = overlay;
        self
    }

    /// 適用可否の列を置き換える。
    #[must_use]
    pub fn conditional(mut self, conditional: Vec<bool>) -> WorkflowExecutionStateBuilder {
        self.state.conditional = conditional;
        self
    }

    /// checkbox 列を置き換える。
    #[must_use]
    pub fn checkbox(mut self, checkbox: Vec<CheckboxState>) -> WorkflowExecutionStateBuilder {
        self.state.checkbox = checkbox;
        self
    }

    /// カーソル位置を置き換える。
    #[must_use]
    pub const fn cursor(mut self, cursor: usize) -> WorkflowExecutionStateBuilder {
        self.state.cursor = cursor;
        self
    }

    /// ワークフロー全体の状態を置き換える。
    #[must_use]
    pub const fn status(mut self, status: Status) -> WorkflowExecutionStateBuilder {
        self.state.status = status;
        self
    }

    /// park マーカーの位置を置き換える。
    #[must_use]
    pub const fn parked_at(mut self, parked_at: Option<usize>) -> WorkflowExecutionStateBuilder {
        self.state.parked_at = parked_at;
        self
    }

    /// 自律モードを置き換える。
    #[must_use]
    pub const fn autonomy(mut self, autonomy: AutonomyMode) -> WorkflowExecutionStateBuilder {
        self.state.autonomy = autonomy;
        self
    }

    /// ゲート承認履歴を置き換える。
    #[must_use]
    pub fn approved(mut self, approved: Vec<bool>) -> WorkflowExecutionStateBuilder {
        self.state.approved = approved;
        self
    }

    /// 差し戻し回数の列を置き換える。
    #[must_use]
    pub fn revision_count(mut self, revision_count: Vec<u32>) -> WorkflowExecutionStateBuilder {
        self.state.revision_count = revision_count;
        self
    }

    /// 順序番号を置き換える。
    #[must_use]
    pub const fn seq_nr(mut self, seq_nr: usize) -> WorkflowExecutionStateBuilder {
        self.state.seq_nr = seq_nr;
        self
    }

    /// 最終更新時刻を置き換える。
    #[must_use]
    pub const fn last_updated_at(
        mut self,
        last_updated_at: DateTime<Utc>,
    ) -> WorkflowExecutionStateBuilder {
        self.state.last_updated_at = last_updated_at;
        self
    }

    /// 状態の写し (memento) を取り出す (検証はしない)。
    #[must_use]
    pub fn build(self) -> WorkflowExecutionState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        AutonomyMode, IntentId, StageDisplay, StageEntry, StageIndex, Status,
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

    fn builder() -> WorkflowExecutionStateBuilder {
        WorkflowExecutionStateBuilder::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            entries(),
        )
    }

    #[test]
    fn the_builder_defaults_derive_the_birth_state_from_the_stage_entries() {
        let state = builder().build();
        assert_eq!(state.stages(), entries().as_slice());
        assert_eq!(state.plan(), [PlanAction::Execute, PlanAction::Execute]);
        assert_eq!(state.overlay(), [PlanAction::Execute, PlanAction::Execute]);
        assert_eq!(state.conditional(), [false, false]);
        assert_eq!(
            state.checkbox(),
            [CheckboxState::InProgress, CheckboxState::Pending]
        );
        assert_eq!(state.approved(), [false, false]);
        assert_eq!(state.revision_count(), [0, 0]);
        assert_eq!(state.cursor(), StageIndex::new(0));
        assert_eq!(state.status(), Status::Running);
        assert_eq!(state.parked_at(), None);
        assert_eq!(state.autonomy(), AutonomyMode::Gated);
        assert_eq!(state.seq_nr(), 1);
        assert_eq!(state.last_updated_at(), DateTime::UNIX_EPOCH);
    }

    #[test]
    fn the_identity_attributes_are_carried_verbatim() {
        let state = builder().build();
        assert_eq!(
            state.intent_id().as_str(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(state.definition_id().as_str(), "claude");
        assert_eq!(
            state.definition_revision().as_str(),
            format!("sha256:{}", "0".repeat(64))
        );
    }

    #[test]
    fn every_mutable_attribute_can_be_overridden() {
        let state = builder()
            .overlay(vec![PlanAction::Execute, PlanAction::Skip])
            .checkbox(vec![CheckboxState::Completed, CheckboxState::Pending])
            .cursor(1)
            .status(Status::Completed)
            .parked_at(Some(1))
            .autonomy(AutonomyMode::Autonomous)
            .approved(vec![false, true])
            .revision_count(vec![0, 3])
            .seq_nr(9)
            .last_updated_at(DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5))
            .build();
        assert_eq!(state.overlay(), [PlanAction::Execute, PlanAction::Skip]);
        assert_eq!(
            state.checkbox(),
            [CheckboxState::Completed, CheckboxState::Pending]
        );
        assert_eq!(state.cursor(), StageIndex::new(1));
        assert_eq!(state.status(), Status::Completed);
        assert_eq!(state.parked_at(), Some(StageIndex::new(1)));
        assert_eq!(state.autonomy(), AutonomyMode::Autonomous);
        assert_eq!(state.approved(), [false, true]);
        assert_eq!(state.revision_count(), [0, 3]);
        assert_eq!(state.seq_nr(), 9);
        assert_eq!(
            state.last_updated_at(),
            DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5)
        );
    }

    #[test]
    fn the_static_plan_and_conditional_lists_can_be_supplied_independently() {
        // C6 の行としては独立した列。集約側 `from_state` が stages との整合を検査する。
        let state = builder()
            .plan(vec![PlanAction::Skip, PlanAction::Execute])
            .conditional(vec![true, false])
            .build();
        assert_eq!(state.plan(), [PlanAction::Skip, PlanAction::Execute]);
        assert_eq!(state.conditional(), [true, false]);
    }

    #[test]
    fn states_compare_by_value() {
        assert_eq!(builder().build(), builder().build());
        assert_ne!(builder().build(), builder().seq_nr(2).build());
    }

    #[test]
    fn an_empty_stage_list_still_builds_and_is_rejected_later_by_the_aggregate() {
        // ビルダーは形を検証しない (検査点は `from_state` の 1 か所 — security-design §2)。
        let state = WorkflowExecutionStateBuilder::new(
            IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            Vec::new(),
        )
        .build();
        assert!(state.stages().is_empty());
        assert!(state.checkbox().is_empty());
    }
}
