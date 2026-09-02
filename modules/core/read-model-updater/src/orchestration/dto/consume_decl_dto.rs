//! `ConsumeDecl` の永続化 DTO (**読む側**) — 上流成果物の消費宣言の行の形。

use core_command_domain::workflow_definition::ConsumeDecl;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{project_type_of, project_type_spelling};

/// 上流成果物の消費宣言の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ConsumeDeclDto {
    artifact: String,
    required: bool,
    conditional_on: Option<String>,
}

impl ConsumeDeclDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(decl: &ConsumeDecl) -> ConsumeDeclDto {
        ConsumeDeclDto {
            artifact: decl.artifact().to_string(),
            required: decl.required(),
            conditional_on: decl
                .conditional_on()
                .map(|value| project_type_spelling(value).to_string()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<ConsumeDecl, DtoDecodeError> {
        let conditional_on = match &self.conditional_on {
            None => None,
            Some(raw) => Some(project_type_of(raw)?),
        };
        Ok(ConsumeDecl::new(
            self.artifact.clone(),
            self.required,
            conditional_on,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workflow_definition::BrownfieldGreenfield;

    #[test]
    fn the_declaration_survives_the_round_trip() {
        let decl = ConsumeDecl::new("design", true, Some(BrownfieldGreenfield::Brownfield));
        assert_eq!(ConsumeDeclDto::of(&decl).to_domain().unwrap(), decl);

        let bare = ConsumeDecl::new("design", false, None);
        assert_eq!(ConsumeDeclDto::of(&bare).to_domain().unwrap(), bare);
    }

    #[test]
    fn an_unknown_project_type_spelling_is_refused() {
        let dto = ConsumeDeclDto {
            artifact: "design".to_string(),
            required: true,
            conditional_on: Some("Brownfield".to_string()),
        };
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("project_type", "Brownfield")
        );
    }
}
