//! `RedefinedDto` — 改訂の payload。イベント識別子・系譜 ID・改訂後の内容版と内容。

use serde::{Deserialize, Serialize};

use super::workflow_definition_dto::DefinitionContentDto;

/// 改訂の payload。**フィールド名と並びが契約**である。
///
/// 系譜 ID を `aggregate_id` として載せるようになったのは b40 である — かつては
/// 「どの集約に起きたかは行の `aid` が持つ」として識別子を持たなかったが、ドメインイベントは
/// エンティティの一種なので自分で述べる (オーナー裁定 2026-09-02)。復号境界はこれと行の
/// `aid` を照合できるようになった。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedefinedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) revision: String,
    pub(super) content: DefinitionContentDto,
}
