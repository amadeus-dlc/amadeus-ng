//! `RedefinedDto` — 改訂の payload の行の形 (**読む側**)。改訂後の内容版と内容。

use core_command_domain::workflow_definition::{DefinitionRevision, Redefined};
use serde::{Deserialize, Serialize};

use super::definition_content_dto::DefinitionContentDto;
use super::dto_decode_error::DtoDecodeError;

/// 改訂の payload — 改訂後の内容版と内容。**フィールド名と並びが契約**である。
///
/// 系譜 ID は載らない — 改訂は既存のストリームに追記される事実であり、どの集約に起きたかは
/// ジャーナル行の `aid` 列が持つ (`coding-rules/aggregate-references.md`)。したがって読取は
/// 行の `aid` を定義 id として使う。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedefinedDto {
    revision: String,
    content: DefinitionContentDto,
}

impl RedefinedDto {
    /// 改訂記録から DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(redefined: &Redefined) -> RedefinedDto {
        RedefinedDto {
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
            DefinitionRevision::parse(&self.revision)
                .map_err(|_| DtoDecodeError::malformed("revision", self.revision.clone()))?,
            graph,
            grid,
            scopes,
        ))
    }
}
