//! `Unparked` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::Unparked;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of};

/// `Unparked` の材料 — ドメインの材料は無いが、識別子は運ぶ。
///
/// かつては単位変種 (`"Unparked"` という裸の文字列) だった。ドメインイベントは
/// エンティティの一種であり、材料の有無にかかわらず `id` と `aggregate_id` を持つので、
/// 行の形も構造体になった (オーナー裁定 2026-09-02)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnparkedDto {
    id: String,
    aggregate_id: String,
}

impl UnparkedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Unparked) -> UnparkedDto {
        UnparkedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 識別子の綴りが文法を外れる。
    pub(super) fn to_domain(&self) -> Result<Unparked, DtoDecodeError> {
        Ok(Unparked::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
        ))
    }
}
