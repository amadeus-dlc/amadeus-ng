//! `CreatedDto` — `IntentEvent::Created` の材料 (intent ジャーナル行の payload)。

use core_command_domain::orchestration::{Created, Intent, IntentEventId, IntentId, StageEntry};
use core_command_domain::workflow_definition::{DefinitionRevision, WorkflowDefinitionId};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_dto::{StageEntryDto, StartRequestDto, WorkspaceScanDto};

/// intent の誕生記録の行の形。**フィールド名と並びが契約**である。
///
/// 先頭 2 つは `id` (イベント自身の識別子) と `aggregate_id` (どの集約の事実か) —
/// ドメインイベントはエンティティの一種だからである (オーナー裁定 2026-09-02)。以降は
/// 誕生の材料で、スナップショット行 [`IntentDto`](super::IntentDto) と同じ部品 DTO
/// (`StartRequestDto` / `StageEntryDto` / `WorkspaceScanDto`) を共有するので、面ごとの
/// 綴りの乖離が構造的に起きない。
///
/// スナップショット行と分けたのは、あちらの `id` が**集約の**識別子だからである — 同じ
/// キー名に別の意味を載せると、行を読む人にも復号にも嘘をつくことになる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) definition_id: String,
    pub(super) definition_revision: String,
    pub(super) start_request: StartRequestDto,
    pub(super) stages: Vec<StageEntryDto>,
    pub(super) scan: WorkspaceScanDto,
    /// 鋳造の発生時刻 (集約 `Intent::created_at` の写し)。
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

impl CreatedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    ///
    /// 発生時刻は封筒の持ち物なので呼出側 (Repository の `store`) が集約から渡す。
    #[must_use]
    pub(super) fn of(created: &Created, occurred_at: chrono::DateTime<chrono::Utc>) -> CreatedDto {
        // 誕生の材料は集約の全状態と同一なので、内容の綴りは集約の読取面から組む。
        let intent = Intent::from((created.clone(), occurred_at));
        CreatedDto {
            id: created.id().as_str().to_string(),
            aggregate_id: created.aggregate_id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            start_request: StartRequestDto::of(&intent),
            stages: intent.stages().iter().map(StageEntryDto::of).collect(),
            scan: WorkspaceScanDto::of(intent.scan()),
            created_at: occurred_at,
        }
    }

    /// 検査付き再構成コンストラクタへ渡して誕生記録へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、計画の不変条件違反は `InvariantViolation`
    /// を返す。後者をここで止めないと、破れた計画が集約の再構成まで届いてクラッシュする
    /// (再構成は失敗を返さない — オーナー裁定 2026-08-30)。
    pub(super) fn to_domain(&self) -> Result<Created, DtoDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(StageEntryDto::to_domain)
            .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
        // 計画そのものの不変条件はドメインが持つ (`StageEntry::check_plan`) — 判断を DTO に
        // 複製せず呼ぶだけにする (`Started` 面と同じ規律 — b40 で intent 面にも揃えた)。
        StageEntry::check_plan(&stages).map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(Created::new(
            IntentEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
            IntentId::parse(&self.aggregate_id).map_err(|_| {
                DtoDecodeError::malformed("aggregate_id", self.aggregate_id.clone())
            })?,
            WorkflowDefinitionId::parse(&self.definition_id).map_err(|_| {
                DtoDecodeError::malformed("definition_id", self.definition_id.clone())
            })?,
            DefinitionRevision::parse(&self.definition_revision).map_err(|_| {
                DtoDecodeError::malformed("definition_revision", self.definition_revision.clone())
            })?,
            self.start_request.to_domain(),
            stages,
            self.scan.to_domain()?,
        ))
    }
}
