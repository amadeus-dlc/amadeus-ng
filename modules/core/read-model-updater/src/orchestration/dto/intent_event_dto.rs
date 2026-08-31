//! intent 自身のジャーナル行 `payload` 列の読み戻し — `IntentEvent` の読む側 DTO。
//!
//! 書く側 (command interface-adapter の `IntentEventDto`) と**共有しない**同名の別の型で
//! ある (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。一致は横断適合テストが固定する。
//!
//! `Created` の中身は [`IntentDto`] そのもの — 誕生の材料 = 集約の全状態であり、復号結果は
//! 検査付き再構成を通った [`Intent`] として返す。RMU が状態ファイルの骨格 (全ステージ行・
//! 表示属性・走査結果) を描く材料の正本である (issue #56)。
//!
//! [`Intent`]: core_command_domain::orchestration::Intent

use core_command_domain::orchestration::Intent;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_dto::IntentDto;

/// intent ジャーナル行の形 (外部タグ形 `{"Created":{...}}` — 書く側と同じバイト)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentEventDto {
    /// intent が作られた (誕生の材料 = 集約の全状態)。
    Created(IntentDto),
}

impl IntentEventDto {
    /// intent (誕生記録から起こした集約値) から行の形を組む (書き — テストが行を用意する
    /// ためだけの口。本番の書き手はコマンド側である)。
    #[must_use]
    pub fn of(intent: &Intent) -> IntentEventDto {
        IntentEventDto::Created(IntentDto::of(intent))
    }

    /// 誕生の材料を検査付き再構成で [`Intent`] へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed` を返す。
    ///
    /// [`Intent`]: core_command_domain::orchestration::Intent
    pub fn to_domain(&self) -> Result<Intent, DtoDecodeError> {
        match self {
            IntentEventDto::Created(intent) => intent.to_domain(),
        }
    }
}
