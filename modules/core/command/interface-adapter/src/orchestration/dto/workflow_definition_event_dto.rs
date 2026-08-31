//! `WorkflowDefinitionEvent` の永続化 DTO — 定義ジャーナル行の payload のバイト形。
//!
//! 変種名は**行に書かれて残る綴り**である。増やすのは新しい事実を足すときだけで、既存の
//! 綴りは変えない (変えると既に書かれた行が読めなくなる)。
//!
//! 誕生は [`DefinedDto`]、改訂は [`RedefinedDto`] が張る。内容部分はどちらも
//! [`DefinitionContentDto`] であり、スナップショット行 (`WorkflowDefinitionDto`) もそれを
//! 共有するので、面ごとの乖離が構造的に起きない。
//!
//! **発生時刻は payload に載せない** — 輸送のメタデータは封筒が運ぶ (ADR-010 / B7)。

use core_command_domain::workflow_definition::{
    DefinitionRevision, Redefined, WorkflowDefinitionEvent,
};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::workflow_definition_dto::{DefinedDto, DefinitionContentDto};

/// 定義ジャーナル行の payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowDefinitionEventDto {
    /// 定義が確立された (genesis)。系譜 ID を運ぶのはこの変種だけである。
    Defined(DefinedDto),
    /// 定義が別の内容版へ改訂された。
    Redefined(RedefinedDto),
}

/// 改訂の payload — 改訂後の内容版と内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedefinedDto {
    revision: String,
    content: DefinitionContentDto,
}

impl WorkflowDefinitionEventDto {
    /// ドメインイベントを行の形へ写す (書き)。
    #[must_use]
    pub fn of(event: &WorkflowDefinitionEvent) -> WorkflowDefinitionEventDto {
        match event {
            WorkflowDefinitionEvent::Defined(defined) => {
                WorkflowDefinitionEventDto::Defined(DefinedDto::of(defined))
            }
            WorkflowDefinitionEvent::Redefined(redefined) => {
                WorkflowDefinitionEventDto::Redefined(RedefinedDto {
                    revision: redefined.revision().as_str().to_string(),
                    content: DefinitionContentDto::of(
                        redefined.graph(),
                        redefined.grid(),
                        redefined.scopes(),
                    ),
                })
            }
        }
    }

    /// 行の形からドメインイベントへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子・グラフの不変条件違反。
    pub fn to_domain(&self) -> Result<WorkflowDefinitionEvent, DtoDecodeError> {
        match self {
            WorkflowDefinitionEventDto::Defined(dto) => {
                Ok(WorkflowDefinitionEvent::Defined(dto.to_domain()?))
            }
            WorkflowDefinitionEventDto::Redefined(dto) => {
                let (graph, grid, scopes) = dto.content.to_domain()?;
                Ok(WorkflowDefinitionEvent::Redefined(Redefined::new(
                    DefinitionRevision::parse(&dto.revision)
                        .map_err(|_| DtoDecodeError::malformed("revision", dto.revision.clone()))?,
                    graph,
                    grid,
                    scopes,
                )))
            }
        }
    }
}
