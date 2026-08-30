//! intent ジャーナル行 `payload` 列のバイト形 — `IntentEvent` の永続化 DTO。
//!
//! 実行のジャーナル ([`WireEvent`]) と同じ**外部タグ形**である: 変種名がトップレベルの
//! 唯一のキーになる (`{"Created":{...}}`)。`Created` の中身は [`WireIntent`] そのもの —
//! 誕生の材料と集約の全状態は同一物なので、綴りを別に定義しない (issue #50)。
//!
//! [`WireEvent`]: super::wire_event::WireEvent

use core_command_domain::orchestration::{Intent, IntentEvent};
use serde::{Deserialize, Serialize};

use super::wire_error::WireDecodeError;
use super::wire_intent::WireIntent;

/// intent ジャーナル行の形。**変種名とフィールド名が契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireIntentEvent {
    /// intent が作られた (誕生の材料 = 集約の全状態)。
    Created(WireIntent),
}

impl WireIntentEvent {
    /// ドメインイベントから行の形を組む (書き)。
    ///
    /// `Created` は誕生の材料をそのまま運ぶ — 材料から起こした集約の読取面と同じバイトに
    /// なる (誕生記録の変換 `From<Created>` は全属性の素通しである)。
    #[must_use]
    pub fn of(event: &IntentEvent) -> WireIntentEvent {
        match event {
            IntentEvent::Created(created) => {
                WireIntentEvent::Created(WireIntent::of(&Intent::from(created.clone())))
            }
        }
    }

    /// 行からドメインイベントへ戻す (読み — 検査付き復号を必ず通る)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed` を返す。
    pub fn to_domain(&self) -> Result<IntentEvent, WireDecodeError> {
        match self {
            WireIntentEvent::Created(intent) => Ok(IntentEvent::Created(intent.to_created()?)),
        }
    }
}
