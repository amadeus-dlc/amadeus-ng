//! `DefinedDto` — 誕生記録 (genesis) の行の形。イベント識別子・系譜 ID・内容版・内容。

use core_command_domain::workflow_definition::{
    Defined, DefinitionRevision, WorkflowDefinitionEventId, WorkflowDefinitionId,
};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::workflow_definition_dto::DefinitionContentDto;

/// 誕生記録の行の形。**フィールド名と並びが契約**である。
///
/// 先頭 2 つは `id` (イベント自身の識別子) と `aggregate_id` (どの集約の事実か) —
/// ドメインイベントはエンティティの一種だからである (オーナー裁定 2026-09-02)。内容部分
/// [`DefinitionContentDto`] はスナップショット行 [`WorkflowDefinitionDto`] と共有するので、
/// 面ごとの綴りの乖離が構造的に起きない。
///
/// [`WorkflowDefinitionDto`]: super::WorkflowDefinitionDto
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) revision: String,
    pub(super) content: DefinitionContentDto,
}

impl DefinedDto {
    /// 誕生記録から DTO を組む (書き)。
    #[must_use]
    pub(super) fn of(defined: &Defined) -> DefinedDto {
        DefinedDto {
            id: defined.id().as_str().to_string(),
            aggregate_id: defined.aggregate_id().as_str().to_string(),
            revision: defined.revision().as_str().to_string(),
            content: DefinitionContentDto::of(defined.graph(), defined.grid(), defined.scopes()),
        }
    }

    /// 誕生記録として復号する (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子・不変条件違反。
    pub(super) fn to_domain(&self) -> Result<Defined, DtoDecodeError> {
        let (graph, grid, scopes) = self.content.to_domain()?;
        Ok(Defined::new(
            WorkflowDefinitionEventId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
            WorkflowDefinitionId::parse(&self.aggregate_id).map_err(|_| {
                DtoDecodeError::malformed("aggregate_id", self.aggregate_id.clone())
            })?,
            DefinitionRevision::parse(&self.revision)
                .map_err(|_| DtoDecodeError::malformed("revision", self.revision.clone()))?,
            graph,
            grid,
            scopes,
        ))
    }
}
