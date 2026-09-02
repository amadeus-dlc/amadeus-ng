//! `ScopeMetadata` の永続化 DTO (**読む側**) — スコープメタデータ 1 件の行の形。

use core_command_domain::workflow_definition::ScopeMetadata;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    review_cap_of, review_cap_spelling, skeleton_default_of, skeleton_default_spelling,
};

/// スコープメタデータ 1 件の行の形。**フィールド名と並びが契約**である。
///
/// 各要素が自分の名前 (`name`) を持つので、[`DefinitionContentDto`] 側は鍵で二重に
/// 持たない。
///
/// [`DefinitionContentDto`]: super::definition_content_dto::DefinitionContentDto
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ScopeMetadataDto {
    name: String,
    depth: Option<String>,
    keywords: Vec<String>,
    skeleton: Option<String>,
    review_cap: Option<String>,
    freeform_default: bool,
}

impl ScopeMetadataDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(metadata: &ScopeMetadata) -> ScopeMetadataDto {
        ScopeMetadataDto {
            name: metadata.name().to_string(),
            depth: metadata.depth().map(str::to_string),
            keywords: metadata.keywords().to_vec(),
            skeleton: metadata
                .skeleton()
                .map(|value| skeleton_default_spelling(value).to_string()),
            review_cap: metadata
                .review_cap()
                .map(|value| review_cap_spelling(value).to_string()),
            freeform_default: metadata.freeform_default(),
        }
    }

    /// 検査付き再構成経路へ渡してドメインへ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<ScopeMetadata, DtoDecodeError> {
        let mut metadata = ScopeMetadata::new(&self.name)
            .map_err(|_| DtoDecodeError::malformed("scope_name", self.name.clone()))?;
        if let Some(depth) = &self.depth {
            metadata = metadata.with_depth(depth.clone());
        }
        if !self.keywords.is_empty() {
            metadata = metadata.with_keywords(self.keywords.clone());
        }
        if let Some(skeleton) = &self.skeleton {
            metadata = metadata.with_skeleton(skeleton_default_of(skeleton)?);
        }
        if let Some(review_cap) = &self.review_cap {
            metadata = metadata.with_review_cap(review_cap_of(review_cap)?);
        }
        Ok(metadata.with_freeform_default(self.freeform_default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::workflow_definition::{ReviewCapValue, SkeletonDefault};

    fn saturated() -> ScopeMetadata {
        ScopeMetadata::new("feature")
            .unwrap()
            .with_depth("standard".to_string())
            .with_keywords(vec!["api".to_string(), "endpoint".to_string()])
            .with_skeleton(SkeletonDefault::On)
            .with_review_cap(ReviewCapValue::Advisory)
            .with_freeform_default(true)
    }

    #[test]
    fn every_optional_field_survives_the_round_trip() {
        let metadata = saturated();
        assert_eq!(
            ScopeMetadataDto::of(&metadata).to_domain().unwrap(),
            metadata
        );

        let bare = ScopeMetadata::new("feature").unwrap();
        assert_eq!(ScopeMetadataDto::of(&bare).to_domain().unwrap(), bare);
    }

    #[test]
    fn an_empty_scope_name_is_refused_as_malformed() {
        let mut dto = ScopeMetadataDto::of(&saturated());
        dto.name = String::new();
        assert_eq!(
            dto.to_domain().unwrap_err(),
            DtoDecodeError::malformed("scope_name", "")
        );
    }

    #[test]
    fn an_unknown_closed_set_spelling_is_refused_field_by_field() {
        let mut skeleton = ScopeMetadataDto::of(&saturated());
        skeleton.skeleton = Some("on".to_string());
        assert_eq!(
            skeleton.to_domain().unwrap_err(),
            DtoDecodeError::malformed("skeleton", "on")
        );

        let mut cap = ScopeMetadataDto::of(&saturated());
        cap.review_cap = Some("advisory".to_string());
        assert_eq!(
            cap.to_domain().unwrap_err(),
            DtoDecodeError::malformed("review_cap", "advisory")
        );
    }
}
