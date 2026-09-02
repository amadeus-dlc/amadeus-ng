//! intent ジャーナル行 `payload` 列のバイト形 — `IntentEvent` の永続化 DTO。
//!
//! 実行のジャーナル ([`IntentExecutionEventDto`]) と同じ**外部タグ形**である: 変種名がトップレベルの
//! 唯一のキーになる (`{"Created":{...}}`)。`Created` の中身は [`IntentDto`] そのもの —
//! 誕生の材料と集約の全状態は同一物なので、綴りを別に定義しない (issue #50)。
//!
//! [`IntentExecutionEventDto`]: super::intent_execution_event_dto::IntentExecutionEventDto

use core_command_domain::orchestration::{Intent, IntentEvent};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_dto::IntentDto;

/// intent ジャーナル行の形。**変種名とフィールド名が契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentEventDto {
    /// intent が作られた (誕生の材料 = 集約の全状態)。
    Created(IntentDto),
}

impl IntentEventDto {
    /// ドメインイベントから行の形を組む (書き)。
    ///
    /// `Created` は誕生の材料をそのまま運ぶ — 材料から起こした集約の読取面と同じバイトに
    /// なる (誕生記録の変換 `From<(Created, occurred_at)>` は全属性の素通しである)。
    /// 発生時刻は封筒の持ち物なので呼出側 (Repository の `store`) が集約から渡す。
    #[must_use]
    pub fn of(event: &IntentEvent, occurred_at: chrono::DateTime<chrono::Utc>) -> IntentEventDto {
        match event {
            IntentEvent::Created(created) => IntentEventDto::Created(IntentDto::of(&Intent::from(
                (created.clone(), occurred_at),
            ))),
        }
    }

    /// 行からドメインイベントへ戻す (読み — 検査付き復号を必ず通る)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed` を返す。
    pub fn to_domain(&self) -> Result<IntentEvent, DtoDecodeError> {
        match self {
            IntentEventDto::Created(intent) => Ok(IntentEvent::Created(intent.to_created()?)),
        }
    }
}
