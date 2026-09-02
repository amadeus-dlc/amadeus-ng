//! intent ジャーナル行 `payload` 列のバイト形 — `IntentEvent` の永続化 DTO。
//!
//! 実行のジャーナル ([`IntentExecutionEventDto`]) と同じ**外部タグ形**である: 変種名がトップレベルの
//! 唯一のキーになる (`{"Created":{...}}`)。中身は [`CreatedDto`] で、先頭に `id` (イベント
//! 自身の識別子) と `aggregate_id` (どの集約の事実か) を持ち、以降は誕生の材料 = 集約の全状態
//! である。内容の綴りはスナップショット行 [`IntentDto`](super::IntentDto) と部品 DTO を共有
//! するので、面ごとの乖離は起きない (issue #50 の意図を保ったまま b40 で識別子を分けた)。
//!
//! [`IntentExecutionEventDto`]: super::intent_execution_event_dto::IntentExecutionEventDto

use core_command_domain::orchestration::IntentEvent;
use serde::{Deserialize, Serialize};

use super::created_dto::CreatedDto;
use super::dto_decode_error::DtoDecodeError;

/// intent ジャーナル行の形。**変種名とフィールド名が契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentEventDto {
    /// intent が作られた (誕生の材料 = 集約の全状態 + イベント自身の識別子)。
    Created(CreatedDto),
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
            IntentEvent::Created(created) => {
                IntentEventDto::Created(CreatedDto::of(created, occurred_at))
            }
        }
    }

    /// 行からドメインイベントへ戻す (読み — 検査付き復号を必ず通る)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed` を返す。
    pub fn to_domain(&self) -> Result<IntentEvent, DtoDecodeError> {
        match self {
            IntentEventDto::Created(created) => Ok(IntentEvent::Created(created.to_domain()?)),
        }
    }
}
