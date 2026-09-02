//! 定義ジャーナル行 `payload` 列の読み戻し — `WorkflowDefinitionEvent` の読む側 DTO。
//!
//! 書く側 (command interface-adapter の `WorkflowDefinitionEventDto`) と**共有しない**同名の
//! 別の型である (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。一致は横断適合テスト
//! (`journal_protocol_conformance`) が固定する。
//!
//! 変種名は**行に書かれて残る綴り**である。誕生は [`DefinedDto`]、改訂は [`RedefinedDto`] が
//! 張り、内容部分はどちらも [`DefinitionContentDto`] である。
//!
//! **発生時刻は payload に載らない** — 輸送のメタデータは封筒 (行の列) が運ぶ
//! (ADR-010 / B7)。
//!
//! [`DefinitionContentDto`]: super::definition_content_dto::DefinitionContentDto

use core_command_domain::workflow_definition::WorkflowDefinitionEvent;
use serde::{Deserialize, Serialize};

use super::defined_dto::DefinedDto;
use super::dto_decode_error::DtoDecodeError;
use super::redefined_dto::RedefinedDto;

/// 定義ジャーナル行の payload (外部タグ形 `{"Defined":{...}}` — 書く側と同じバイト)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowDefinitionEventDto {
    /// 定義が確立された (genesis)。系譜 ID を運ぶのはこの変種だけである。
    Defined(DefinedDto),
    /// 定義が別の内容版へ改訂された。
    Redefined(RedefinedDto),
}

impl WorkflowDefinitionEventDto {
    /// ドメインイベントを行の形へ写す (書き — テストが行を用意するためだけの口。本番の
    /// 書き手はコマンド側である)。
    #[must_use]
    pub fn of(event: &WorkflowDefinitionEvent) -> WorkflowDefinitionEventDto {
        match event {
            WorkflowDefinitionEvent::Defined(defined) => {
                WorkflowDefinitionEventDto::Defined(DefinedDto::of(defined))
            }
            WorkflowDefinitionEvent::Redefined(redefined) => {
                WorkflowDefinitionEventDto::Redefined(RedefinedDto::of(redefined))
            }
        }
    }

    /// 行の形からドメインイベントへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、グラフの不変条件違反は
    /// `InvariantViolation` を返す。
    pub fn to_domain(&self) -> Result<WorkflowDefinitionEvent, DtoDecodeError> {
        match self {
            WorkflowDefinitionEventDto::Defined(dto) => {
                Ok(WorkflowDefinitionEvent::Defined(dto.to_domain()?))
            }
            WorkflowDefinitionEventDto::Redefined(dto) => {
                Ok(WorkflowDefinitionEvent::Redefined(dto.to_domain()?))
            }
        }
    }
}
