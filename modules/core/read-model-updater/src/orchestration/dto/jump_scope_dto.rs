//! `JumpScopeDto` — `Jumped` が運ぶ別 scope の実効計画の材料。

use core_command_domain::orchestration::{JumpScope, StageSlugSet};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;

/// 別 scope の名前と、その scope で EXECUTE の stage (slug の辞書順)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct JumpScopeDto {
    name: String,
    executes: Vec<String>,
}

impl JumpScopeDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(scope: &JumpScope) -> JumpScopeDto {
        JumpScopeDto {
            name: scope.name().to_string(),
            executes: scope.executes().fold_left(Vec::new(), |mut slugs, slug| {
                slugs.push(slug.as_str().to_string());
                slugs
            }),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<JumpScope, DtoDecodeError> {
        let executes = self
            .executes
            .iter()
            .map(|raw| {
                StageSlug::parse(raw).map_err(|_| DtoDecodeError::malformed("scope.executes", raw))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(JumpScope::new(
            self.name.clone(),
            StageSlugSet::new(executes),
        ))
    }
}
