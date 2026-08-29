//! `IntentExecutionSnapshot` — 集約の全状態 12 属性の値オブジェクト (C6 / BR5.2)。
//!
//! 集約 → [`IntentExecution::snapshot`]、集約 ← [`IntentExecution::from_snapshot`] の 1 往復で
//! 永続化境界を渡る。**形の検査はしない** — 検査点は `from_snapshot` の 1 か所に集約する
//! (security-design §2)。
//!
//! **この型は永続化の記述を持たない** (改訂 9 / `coding-rules/domain-persistence-neutrality.md`)。
//! 直列化するのはアダプタ層で、その DTO がこの写しを読み書きする。写しの 12 属性が
//! 「集約の全状態」という**ドメインの語彙**であり、それをどんなバイトにするかは相手方契約
//! なのでここには書かない。復号は必ず `from_snapshot` の検査点を通るので、集約不変条件を
//! 迂回する経路は (アダプタ経由でも) 存在しない。
//!
//! **静的な材料は載らない** (オーナー裁定 2026-08-29 / 改訂 3)。定義参照・解決済み計画・
//! base plan_action・conditional は intent の持ち物であり、集約は `intent_id` で参照する
//! (coding-rules/aggregate-references.md)。旧 16 属性のうち `definition_id` /
//! `definition_revision` / `stages` / `plan` / `conditional` はここから消え、`intent_id` が
//! 加わった。
//!
//! 楽観 version も**この写しに載らない** (ADR-010 / B7)。本家 v3 で版数の正本が
//! `SnapshotEnvelope::version()` (スナップショット行の列) になり、payload 列は純粋な
//! ドメイン内容だけを持つようになったためである。
//!
//! [`IntentExecution::snapshot`]: super::intent_execution::IntentExecution::snapshot
//! [`IntentExecution::from_snapshot`]: super::intent_execution::IntentExecution::from_snapshot

use chrono::{DateTime, Utc};

use super::autonomy_mode::AutonomyMode;
use super::intent_execution_id::IntentExecutionId;
use super::intent_id::IntentId;
use super::status::Status;
use crate::workflow_definition::PlanAction;
use crate::workspace::CheckboxState;

/// ある `seq_nr` 時点の集約の全状態 — **公開の memento**。
///
/// 状態を担うのは集約 [`IntentExecution`] であり、この型はその写し (memento) である
/// （オーナー裁定 2026-08-29）。クレート内私有だった裁定は**改訂 9 で上書き**された —
/// 直列化を担うアダプタ層が正当な消費者になったためである。公開するのは読取アクセサと
/// 検査付きの構築 ([`IntentExecutionSnapshotBuilder`] → [`IntentExecution::from_snapshot`]) だけで、
/// フィールドはクレート内に閉じたままである（`coding-rules/field-visibility.md`）。
///
/// [`IntentExecution`]: super::intent_execution::IntentExecution
/// [`IntentExecution::from_snapshot`]: super::intent_execution::IntentExecution::from_snapshot
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentExecutionSnapshot {
    pub(crate) id: IntentExecutionId,
    pub(crate) intent_id: IntentId,
    pub(crate) overlay: Vec<PlanAction>,
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

impl IntentExecutionSnapshot {
    /// 実行の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionId {
        &self.id
    }

    /// 実行の対象 intent の識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 実効プラン (recompose の上書き結果)。
    #[must_use]
    pub fn overlay(&self) -> &[PlanAction] {
        &self.overlay
    }

    /// checkbox 列。
    #[must_use]
    pub fn checkbox(&self) -> &[CheckboxState] {
        &self.checkbox
    }

    /// カーソル位置 (索引)。
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// ワークフロー全体の状態。
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// park マーカーの位置 (`None` = park していない)。
    #[must_use]
    pub const fn parked_at(&self) -> Option<usize> {
        self.parked_at
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

    /// 差し戻し回数の列。
    #[must_use]
    pub fn revision_count(&self) -> &[u32] {
        &self.revision_count
    }

    /// 適用済みイベント数と一致する順序番号。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 最終更新時刻。
    #[must_use]
    pub const fn last_updated_at(&self) -> &DateTime<Utc> {
        &self.last_updated_at
    }
}

/// [`IntentExecutionSnapshot`] のビルダー。
///
/// 12 属性を 1 つの関数引数列で受け取るのは可読でもリント可能でもないため、`StageNodeBuilder`
/// と同じ house style で組み立てる。既定値は intent から導ける birth 時の状態
/// (`overlay` は計画の写し、`checkbox` は先頭のみ in-progress、`approved` は全 false、
/// `revision_count` は全 0、`cursor` = 0、`status` = running、`parked_at` = なし、
/// `autonomy` = gated、`seq_nr` = 1、`last_updated_at` = Unix epoch)。
/// `last_updated_at` の既定が epoch なのは、birth 時の発生時刻を知っているのは呼出側
/// (`Started` を作った側) だけだからである。
///
/// **公開**である (改訂 9) — 直列化を担うアダプタ層が、復号した 12 属性から写しを組み直す
/// 唯一の入口になるためである。構造体リテラルが現れるのはこのビルダーの中だけで、
/// 検査点は従来どおり `IntentExecution::from_snapshot` の 1 か所である。
#[derive(Debug, Clone)]
pub struct IntentExecutionSnapshotBuilder {
    state: IntentExecutionSnapshot,
}

impl IntentExecutionSnapshotBuilder {
    /// 既定できない 3 点 (実行の識別子・対象 intent の識別子・実効プラン) からビルダーを起こす。
    ///
    /// 残る 9 属性は birth 時の既定値になる — `checkbox` は先頭のみ in-progress、`approved` は
    /// 全 false、`revision_count` は全 0、`cursor` = 0、`status` = running、`parked_at` = なし、
    /// `autonomy` = gated、`seq_nr` = 1、`last_updated_at` = Unix epoch。列の長さは `overlay`
    /// から採る。`last_updated_at` の既定が epoch なのは、birth 時の発生時刻を知っているのは
    /// 呼出側 (`Started` を作った側) だけだからである。
    #[must_use]
    pub fn new(
        id: IntentExecutionId,
        intent_id: IntentId,
        overlay: Vec<PlanAction>,
    ) -> IntentExecutionSnapshotBuilder {
        let count = overlay.len();
        let mut checkbox = vec![CheckboxState::Pending; count];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        IntentExecutionSnapshotBuilder {
            state: IntentExecutionSnapshot {
                id,
                intent_id,
                overlay,
                checkbox,
                cursor: 0,
                status: Status::Running,
                parked_at: None,
                autonomy: AutonomyMode::Gated,
                approved: vec![false; count],
                revision_count: vec![0; count],
                seq_nr: 1,
                last_updated_at: DateTime::UNIX_EPOCH,
            },
        }
    }

    /// checkbox 列を置き換える。
    #[must_use]
    pub fn checkbox(mut self, checkbox: Vec<CheckboxState>) -> IntentExecutionSnapshotBuilder {
        self.state.checkbox = checkbox;
        self
    }

    /// カーソル位置を置き換える。
    #[must_use]
    pub const fn cursor(mut self, cursor: usize) -> IntentExecutionSnapshotBuilder {
        self.state.cursor = cursor;
        self
    }

    /// ワークフロー全体の状態を置き換える。
    #[must_use]
    pub const fn status(mut self, status: Status) -> IntentExecutionSnapshotBuilder {
        self.state.status = status;
        self
    }

    /// park マーカーの位置を置き換える。
    #[must_use]
    pub const fn parked_at(mut self, parked_at: Option<usize>) -> IntentExecutionSnapshotBuilder {
        self.state.parked_at = parked_at;
        self
    }

    /// 自律モードを置き換える。
    #[must_use]
    pub const fn autonomy(mut self, autonomy: AutonomyMode) -> IntentExecutionSnapshotBuilder {
        self.state.autonomy = autonomy;
        self
    }

    /// ゲート承認履歴を置き換える。
    #[must_use]
    pub fn approved(mut self, approved: Vec<bool>) -> IntentExecutionSnapshotBuilder {
        self.state.approved = approved;
        self
    }

    /// 差し戻し回数の列を置き換える。
    #[must_use]
    pub fn revision_count(mut self, revision_count: Vec<u32>) -> IntentExecutionSnapshotBuilder {
        self.state.revision_count = revision_count;
        self
    }

    /// 順序番号を置き換える。
    #[must_use]
    pub const fn seq_nr(mut self, seq_nr: usize) -> IntentExecutionSnapshotBuilder {
        self.state.seq_nr = seq_nr;
        self
    }

    /// 最終更新時刻を置き換える。
    #[must_use]
    pub const fn last_updated_at(
        mut self,
        last_updated_at: DateTime<Utc>,
    ) -> IntentExecutionSnapshotBuilder {
        self.state.last_updated_at = last_updated_at;
        self
    }

    /// 状態の写し (memento) を取り出す (検証はしない)。
    ///
    /// 検査点は `IntentExecution::from_snapshot` の 1 か所に集約したままにする
    /// (security-design §2) — 呼出側はこの写しを `from_snapshot` / `TryFrom` に渡して集約を得る。
    #[must_use]
    pub fn build(self) -> IntentExecutionSnapshot {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        Intent, IntentExecution, IntentId, StageDisplay, StageEntry, StageIndex, StartRequest,
        WorkspaceScan,
    };
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

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

    fn intent() -> Intent {
        Intent::from_material(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build it"),
            entries(),
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
    }

    fn builder() -> IntentExecutionSnapshotBuilder {
        let intent = intent();
        IntentExecutionSnapshotBuilder::new(execution_id(), intent.id().clone(), overlay(&intent))
    }

    /// intent の解決済み計画をそのまま実効プランへ写す (birth 時の overlay)。
    fn overlay(intent: &Intent) -> Vec<PlanAction> {
        intent
            .stages()
            .iter()
            .map(StageEntry::plan_action)
            .collect()
    }

    /// 組んだ写しを検査点 (`IntentExecution::from_snapshot`) に通して集約を得る。
    ///
    /// 観測は集約の面で行う — 写しは公開になったが、不変条件を語れるのは集約だけである。
    fn built(builder: IntentExecutionSnapshotBuilder) -> IntentExecution {
        IntentExecution::from_snapshot(builder.build()).unwrap()
    }

    /// 索引から `StageIndex` を作る (集約だけが範囲を知っている)。
    fn at(execution: &IntentExecution, index: usize) -> StageIndex {
        execution.stage_index(index).unwrap()
    }

    #[test]
    fn the_builder_defaults_derive_the_birth_state_from_the_intent() {
        let execution = built(builder());
        assert_eq!(execution.id(), &execution_id());
        assert_eq!(execution.intent_id(), intent().id());
        assert_eq!(execution.stage_count(), 2);
        assert_eq!(
            execution.effective_plan(at(&execution, 0)),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            execution.effective_plan(at(&execution, 1)),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            execution.checkbox(at(&execution, 0)),
            Some(CheckboxState::InProgress)
        );
        assert_eq!(
            execution.checkbox(at(&execution, 1)),
            Some(CheckboxState::Pending)
        );
        assert_eq!(execution.approved(at(&execution, 0)), Some(false));
        assert_eq!(execution.revision_count(at(&execution, 0)), Some(0));
        assert_eq!(execution.cursor(), StageIndex::new(0));
        assert_eq!(execution.status(), Status::Running);
        assert_eq!(execution.parked_at(), None);
        assert_eq!(execution.autonomy(), AutonomyMode::Gated);
        assert_eq!(execution.seq_nr(), 1);
        assert_eq!(*execution.last_updated_at(), DateTime::UNIX_EPOCH);
    }

    #[test]
    fn every_mutable_attribute_can_be_overridden() {
        let intent = intent();
        let execution = built(
            IntentExecutionSnapshotBuilder::new(
                execution_id(),
                intent.id().clone(),
                vec![PlanAction::Execute, PlanAction::Skip],
            )
            .checkbox(vec![CheckboxState::Completed, CheckboxState::Pending])
            .cursor(1)
            .status(Status::Completed)
            .parked_at(Some(1))
            .autonomy(AutonomyMode::Autonomous)
            .approved(vec![false, true])
            .revision_count(vec![0, 3])
            .seq_nr(9)
            .last_updated_at(DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5)),
        );
        assert_eq!(
            execution.effective_plan(at(&execution, 1)),
            Some(PlanAction::Skip)
        );
        assert_eq!(
            execution.checkbox(at(&execution, 0)),
            Some(CheckboxState::Completed)
        );
        assert_eq!(execution.cursor(), StageIndex::new(1));
        assert_eq!(execution.status(), Status::Completed);
        assert_eq!(execution.parked_at(), Some(StageIndex::new(1)));
        assert_eq!(execution.autonomy(), AutonomyMode::Autonomous);
        assert_eq!(execution.approved(at(&execution, 1)), Some(true));
        assert_eq!(execution.revision_count(at(&execution, 1)), Some(3));
        assert_eq!(execution.seq_nr(), 9);
        assert_eq!(
            *execution.last_updated_at(),
            DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(5)
        );
    }

    #[test]
    fn executions_built_from_the_same_attributes_compare_equal() {
        assert_eq!(built(builder()), built(builder()));
        assert_ne!(built(builder()), built(builder().seq_nr(2)));
    }
}
