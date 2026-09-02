//! `RedefinedDto` — 改訂の payload の行の形 (**読む側**)。イベント識別子・系譜 ID・内容版・内容。

use core_command_domain::workflow_definition::{
    DefinitionRevision, Redefined, WorkflowDefinitionEventId, WorkflowDefinitionId,
};
use serde::{Deserialize, Serialize};

use super::definition_content_dto::DefinitionContentDto;
use super::dto_decode_error::DtoDecodeError;

/// 改訂の payload。**フィールド名と並びが契約**である。
///
/// 系譜 ID を `aggregate_id` として載せるようになったのは b40 である — かつては
/// 「どの集約に起きたかは行の `aid` が持つ」として識別子を持たず、読取は行の `aid` を
/// 定義 id として使っていた。ドメインイベントはエンティティの一種なので自分で述べる
/// (オーナー裁定 2026-09-02)。復号境界はこれと行の `aid` を照合する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedefinedDto {
    id: String,
    aggregate_id: String,
    revision: String,
    content: DefinitionContentDto,
}

impl RedefinedDto {
    /// 改訂記録から DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(redefined: &Redefined) -> RedefinedDto {
        RedefinedDto {
            id: redefined.id().as_str().to_string(),
            aggregate_id: redefined.aggregate_id().as_str().to_string(),
            revision: redefined.revision().as_str().to_string(),
            content: DefinitionContentDto::of(
                redefined.graph(),
                redefined.grid(),
                redefined.scopes(),
            ),
        }
    }

    /// 改訂記録として復号する (読み)。
    pub(super) fn to_domain(&self) -> Result<Redefined, DtoDecodeError> {
        let (graph, grid, scopes) = self.content.to_domain()?;
        Ok(Redefined::new(
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
