//! `IntentEvent` — intent 集約に起きた事実 (現在は genesis の 1 変種)。
//!
//! [`Intent`] は**集約**である (オーナー裁定 2026-08-30 — 静的で変異が現状無いだけで、
//! [`WorkflowDefinition`] と同じ類型)。集約のファクトリは **(集約インスタンス, 誕生イベント)
//! の対を返す**ことが必須なので (coding-rules/aggregate-commands.md)、intent 側にもイベント
//! 語彙を持たせる。
//!
//! 現スコープでは**ジャーナルへ接続しない** — 型と形だけを規則へ適合させ、`Created` を
//! `store` する `IntentRepository` は U7 (intent-create の実装) の課題である。
//!
//! イベントは**内容 (値) を運ぶ** — 集約はイベント列から `From<Created>` +
//! [`Intent::replay`] で導出する (オーナー裁定 2026-08-30、本家 v3 サンプル同型)。
//!
//! [`Intent`]: super::intent::Intent
//! [`Intent::replay`]: super::intent::Intent::replay
//! [`WorkflowDefinition`]: crate::workflow_definition::WorkflowDefinition

use super::intent_id::IntentId;
use super::stage_entry::StageEntry;
use super::start_request::StartRequest;
use super::workspace_scan::WorkspaceScan;
use crate::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};

/// intent 集約に起きた事実。現在は genesis の 1 変種だけである。
///
/// 変異 (計画の再解決・依頼文の訂正など) が要件化したら、差分を運ぶ変種がここへ増える。
/// `#[non_exhaustive]` は付けない — 変種の追加は設計事項であり、消費側の網羅 match が
/// 落ちること自体が検出手段である ([`IntentExecutionEvent`] と同じ方針)。
///
/// [`IntentExecutionEvent`]: super::intent_execution_event::IntentExecutionEvent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentEvent {
    /// intent が作られた。
    Created(Created),
}

/// `Created` のペイロード — 作られた時点の intent の**内容 (値)** を運ぶ。
///
/// 本家 v3 のイベントペイロードと同型 — イベントは純粋なドメイン内容 (値) だけを運び、
/// 集約インスタンスを埋め込まない (`UserAccountEvent::Created { name }` の形)。集約を
/// 埋め込むと「イベントを復号するには集約が要り、集約はイベントからしか作れない」という
/// 循環が生じ、イベントからのリプレイが成立しない (オーナー裁定 2026-08-30)。
///
/// intent は静的 (Always Valid・変異メソッドなし) なので、**全属性がそのまま誕生の材料**で
/// ある。この誕生記録から集約を起こすのは [`Intent`] の `From<Created>` 変換であり、
/// リプレイのスナップショット種はそこから得る。
///
/// [`Intent`]: super::intent::Intent
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub(crate) id: IntentId,
    pub(crate) definition_id: WorkflowDefinitionId,
    pub(crate) definition_revision: DefinitionRevision,
    pub(crate) start_request: StartRequest,
    pub(crate) stages: Vec<StageEntry>,
    pub(crate) scan: WorkspaceScan,
}

impl Created {
    /// 誕生の材料を束ねる (検査なし — イベントは記録であり、集約への変換時に検査される)。
    #[must_use]
    pub const fn new(
        id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        start_request: StartRequest,
        stages: Vec<StageEntry>,
        scan: WorkspaceScan,
    ) -> Created {
        Created {
            id,
            definition_id,
            definition_revision,
            start_request,
            stages,
            scan,
        }
    }

    /// 作られた intent の識別子。
    #[must_use]
    pub const fn id(&self) -> &IntentId {
        &self.id
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

    /// 人間の要求 (逐語保持)。
    #[must_use]
    pub const fn start_request(&self) -> &StartRequest {
        &self.start_request
    }

    /// 解決済み計画 (文書順)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }

    /// ワークスペース走査の結果。
    #[must_use]
    pub const fn scan(&self) -> &WorkspaceScan {
        &self.scan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::{
        BrownfieldGreenfield, PhaseId, PlanAction, StageNumber, StageSlug,
    };

    use super::super::stage_display::StageDisplay;

    fn created() -> Created {
        let stages = vec![StageEntry::new(
            StageSlug::parse("state-init").unwrap(),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            StageDisplay::new(
                StageNumber::parse("0.1").unwrap(),
                "State Init",
                "orchestrator",
            )
            .unwrap(),
        )];
        Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build the thing"),
            stages,
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        )
    }

    #[test]
    fn the_created_payload_carries_the_birth_material() {
        let created = created();
        assert_eq!(
            created.id().to_string(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(created.definition_id().to_string(), "claude");
        assert_eq!(created.start_request().scope(), "classic");
        assert_eq!(created.stages().len(), 1);
    }

    #[test]
    fn events_built_from_the_same_material_compare_equal() {
        assert_eq!(
            IntentEvent::Created(created()),
            IntentEvent::Created(created())
        );
    }
}
