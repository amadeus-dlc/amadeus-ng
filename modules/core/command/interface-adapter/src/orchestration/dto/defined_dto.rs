//! `DefinedDto` — 誕生記録 (genesis) の行の形。系譜 ID・内容版・内容。
//!
//! 誕生イベント `Defined` の payload であり、
//! [`WorkflowDefinitionDto`](super::WorkflowDefinitionDto) (スナップショット行) の内容部分
//! でもある — 内容の綴りが 1 か所に束なるので、面ごとの乖離が構造的に起きない。

use core_command_domain::workflow_definition::{Defined, DefinitionRevision, WorkflowDefinitionId};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::workflow_definition_dto::DefinitionContentDto;

/// 誕生記録の行の形 — 系譜 ID・内容版・内容。
///
/// 誕生イベント `Defined` の payload であり、スナップショット行の内容部分でもある。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedDto {
    pub(super) id: String,
    pub(super) revision: String,
    pub(super) content: DefinitionContentDto,
}

impl DefinedDto {
    /// 誕生記録から DTO を組む (書き)。
    #[must_use]
    pub(super) fn of(defined: &Defined) -> DefinedDto {
        DefinedDto {
            id: defined.id().as_str().to_string(),
            revision: defined.revision().as_str().to_string(),
            content: DefinitionContentDto::of(defined.graph(), defined.grid(), defined.scopes()),
        }
    }

    /// 誕生記録として復号する (読み — 定義ジャーナル面・スナップショット面の共通経路)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子・不変条件違反。
    pub(super) fn to_domain(&self) -> Result<Defined, DtoDecodeError> {
        let (graph, grid, scopes) = self.content.to_domain()?;
        Ok(Defined::new(
            WorkflowDefinitionId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
            DefinitionRevision::parse(&self.revision)
                .map_err(|_| DtoDecodeError::malformed("revision", self.revision.clone()))?,
            graph,
            grid,
            scopes,
        ))
    }
}
