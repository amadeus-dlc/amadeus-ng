//! `DefinedDto` — 誕生記録 (genesis) の行の形 (**読む側**)。系譜 ID・内容版・内容。

use core_command_domain::workflow_definition::{Defined, DefinitionRevision, WorkflowDefinitionId};
use serde::{Deserialize, Serialize};

use super::definition_content_dto::DefinitionContentDto;
use super::dto_decode_error::DtoDecodeError;

/// 誕生記録の行の形 — 系譜 ID・内容版・内容。**フィールド名と並びが契約**である。
///
/// 書く側ではスナップショット行 (`WorkflowDefinitionDto`) の内容部分でもあるが、RMU は
/// スナップショット行を読まないので、この側にはジャーナル面しか無い
/// (`dto/mod.rs` の「スナップショットは読まない」)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinedDto {
    id: String,
    revision: String,
    content: DefinitionContentDto,
}

impl DefinedDto {
    /// 誕生記録から DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(defined: &Defined) -> DefinedDto {
        DefinedDto {
            id: defined.id().as_str().to_string(),
            revision: defined.revision().as_str().to_string(),
            content: DefinitionContentDto::of(defined.graph(), defined.grid(), defined.scopes()),
        }
    }

    /// 誕生記録として復号する (読み)。
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
