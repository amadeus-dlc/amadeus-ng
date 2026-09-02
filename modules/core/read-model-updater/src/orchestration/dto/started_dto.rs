//! `Started` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::{IntentId, StageEntry, Started};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_dto::StageEntryDto;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of};

/// `Started` の材料 — genesis の 3 点 (実行 id・intent id・解決済み計画)。
///
/// 計画の写しを運ぶのは、実行の歴史が自ストリームだけで再生できるための条件である
/// (`coding-rules/aggregate-commands.md`)。1 要素の綴りは `IntentDto` の `StageEntryDto` を
/// 共有するので、intent 面と同じバイトになる。書き手 (コマンド側アダプタ) の
/// `StartedDto` とワイヤ形式が一致していることは横断適合テストが固定する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedDto {
    id: String,
    aggregate_id: String,
    intent_id: String,
    stages: Vec<StageEntryDto>,
}

impl StartedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &Started) -> StartedDto {
        StartedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            intent_id: payload.intent_id().as_str().to_string(),
            stages: payload.stages().iter().map(StageEntryDto::of).collect(),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 識別子・計画の綴りが文法を外れる、または計画そのものが不変条件を破る。
    pub(super) fn to_domain(&self) -> Result<Started, DtoDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(StageEntryDto::to_domain)
            .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
        // 計画そのものの不変条件はドメインが持つ ([`StageEntry::check_plan`]) —
        // 判断を DTO に複製せず呼ぶだけにする。ここで止めないと、破れた計画が集約の
        // 再構成まで届いてクラッシュする (再構成は失敗を返さない)。
        StageEntry::check_plan(&stages).map_err(|_| DtoDecodeError::InvariantViolation)?;
        Ok(Started::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            IntentId::parse(&self.intent_id)
                .map_err(|_| DtoDecodeError::malformed("intent_id", &self.intent_id))?,
            stages,
        ))
    }
}
