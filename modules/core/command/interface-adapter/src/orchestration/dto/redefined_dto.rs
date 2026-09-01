//! `RedefinedDto` — 改訂の payload。改訂後の内容版と内容。

use serde::{Deserialize, Serialize};

use super::workflow_definition_dto::DefinitionContentDto;

/// 改訂の payload — 改訂後の内容版と内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedefinedDto {
    pub(super) revision: String,
    pub(super) content: DefinitionContentDto,
}
