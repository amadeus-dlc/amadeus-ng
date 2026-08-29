//! `IntentExecutionSnapshot` — 集約の全状態 12 属性の値オブジェクト (C6 / BR5.2)。
//!
//! 集約 → [`IntentExecution::snapshot`]、集約 ← [`IntentExecution::from_snapshot`] の 1 往復で
//! 永続化境界を渡る。**形の検査はしない** — 検査点は `from_snapshot` の 1 か所に集約する
//! (security-design §2)。
//!
//! スナップショットの直列化は**この写しを経由する** (オーナー裁定 2026-08-27 (A)):
//! `IntentExecution` は `#[serde(into / try_from)]` でこの型へ委ね、復号は必ず `from_snapshot`
//! の検査点を通る。したがって直列化形式の正本はこの 12 属性であり、復号が集約不変条件を
//! 迂回する経路は存在しない。
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
use serde::{Deserialize, Serialize};

use super::autonomy_mode::AutonomyMode;
use super::intent_execution_id::IntentExecutionId;
use super::intent_id::IntentId;
use super::status::Status;
use crate::workflow_definition::PlanAction;
use crate::workspace::CheckboxState;

/// ある `seq_nr` 時点の集約の全状態 — **クレート内私有の memento**。
///
/// 状態を担うのは集約 [`IntentExecution`] であり、この型は「State」ではなくその直列化形
/// （memento）である（オーナー裁定 2026-08-29）。クレート外へは出さない — 出口は集約の
/// `Serialize` / `Deserialize` だけで、`IntentExecution` が `into` / `try_from` でここへ委ねる。
/// 属性の綴りと並びを変えると、既に書かれた行を読めなくなる。
///
/// [`IntentExecution`]: super::intent_execution::IntentExecution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IntentExecutionSnapshot {
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
/// 可視性は写しと同じくクレート内私有であり、さらに `#[cfg(test)]` で絞ってある — 本番経路の
/// birth は `IntentExecution::start` が集約を直に起こすので、任意の状態から写しを組む必要が
/// あるのはテストだけだからである (house style: `canon_json::value::arbitrary` と同じ絞り方)。
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct IntentExecutionSnapshotBuilder {
    state: IntentExecutionSnapshot,
}

#[cfg(test)]
impl IntentExecutionSnapshotBuilder {
    /// 実行の識別子と対象 intent から、birth 時の既定値でビルダーを起こす。
    #[must_use]
    pub(crate) fn new(
        id: IntentExecutionId,
        intent: &super::intent::Intent,
    ) -> IntentExecutionSnapshotBuilder {
        let count = intent.stage_count();
        let overlay: Vec<PlanAction> = intent
            .stages()
            .iter()
            .map(crate::orchestration::StageEntry::plan_action)
            .collect();
        let mut checkbox = vec![CheckboxState::Pending; count];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        IntentExecutionSnapshotBuilder {
            state: IntentExecutionSnapshot {
                id,
                intent_id: intent.id().clone(),
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

    /// 実効プランを置き換える。
    #[must_use]
    pub(crate) fn overlay(mut self, overlay: Vec<PlanAction>) -> IntentExecutionSnapshotBuilder {
        self.state.overlay = overlay;
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
    /// 検査点は `IntentExecution::from_snapshot` の 1 か所に集約したままにする
    /// (security-design §2) — 呼出側はこの写しを `from_snapshot` / `TryFrom` に渡して集約を得る。
    #[must_use]
    pub(crate) fn build(self) -> IntentExecutionSnapshot {
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
        Intent::new(
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
        IntentExecutionSnapshotBuilder::new(execution_id(), &intent())
    }

    /// 組んだ写しを検査点 (`IntentExecution::from_snapshot`) に通して集約を得る。
    ///
    /// 観測は集約の面で行う — memento はクレート内私有であり、テストも属性を直に読まない。
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
        let execution = built(
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

    #[test]
    fn the_snapshot_carries_no_static_material_from_the_intent() {
        // 改訂 3 の受入基準 — 集約状態に intent 由来の静的フィールドが残っていないこと。
        // 綴りは行に書かれて残る値なので、属性名を逐語で固定する。
        let snapshot = builder().build();
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの検査 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&snapshot).unwrap();
        for absent in [
            "definition_id",
            "definition_revision",
            "stages",
            "plan",
            "conditional",
        ] {
            assert!(!json.contains(absent), "{absent} は写しに載らない: {json}");
        }
        for present in [
            "id",
            "intent_id",
            "overlay",
            "checkbox",
            "cursor",
            "status",
            "parked_at",
            "autonomy",
            "approved",
            "revision_count",
            "seq_nr",
            "last_updated_at",
        ] {
            assert!(json.contains(present), "{present} は写しに載る: {json}");
        }
    }
}
