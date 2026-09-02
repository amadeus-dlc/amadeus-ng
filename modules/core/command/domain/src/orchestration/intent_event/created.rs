//! `Created` — `IntentEvent::Created` のペイロード。

use crate::orchestration::{IntentEventId, IntentId, StageEntry, StartRequest, WorkspaceScan};
use crate::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};

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
/// [`Intent`]: crate::orchestration::Intent
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// このイベント自身の識別子。`From<Created>` は集約へ持ち込まないので private のまま。
    id: IntentEventId,
    pub(crate) aggregate_id: IntentId,
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
        id: IntentEventId,
        aggregate_id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        start_request: StartRequest,
        stages: Vec<StageEntry>,
        scan: WorkspaceScan,
    ) -> Created {
        Created {
            id,
            aggregate_id,
            definition_id,
            definition_revision,
            start_request,
            stages,
            scan,
        }
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &IntentEventId {
        &self.id
    }

    /// **どの集約の事実か** — 作られた intent の識別子。集約の ID をイベントの id に流用
    /// しない (オーナー裁定 2026-09-02)。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentId {
        &self.aggregate_id
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
