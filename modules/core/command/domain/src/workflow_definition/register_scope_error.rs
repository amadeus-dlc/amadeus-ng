//! `RegisterScopeError` — `CompiledDefinition::register_scope` のガードが拒否する形。

use std::fmt;

use super::stage_slug::StageSlug;

/// スコープの登記を受け付けられない形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterScopeError {
    /// 同じ名前のスコープが既に登記されている (12 §3.3 — 識別ファイルの `name:` 重複は致命)。
    DuplicateScope {
        /// 重複した名前。
        name: String,
    },
    /// `freeform_default` は有効スコープ中 1 つまで (12 §3.3) — 既に別のスコープが持っている。
    FreeformDefaultAlreadyTaken {
        /// 既に `freeform_default` を持つスコープ名。
        holder: String,
    },
    /// 列が、グラフに無いステージを指している。
    UnknownStage {
        /// グラフに無い slug。
        slug: StageSlug,
    },
}

impl fmt::Display for RegisterScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterScopeError::DuplicateScope { name } => write!(f, "duplicate scope {name}"),
            RegisterScopeError::FreeformDefaultAlreadyTaken { holder } => {
                write!(f, "freeform_default already taken by scope {holder}")
            }
            RegisterScopeError::UnknownStage { slug } => {
                write!(f, "unknown stage {} in scope column", slug.as_str())
            }
        }
    }
}

impl std::error::Error for RegisterScopeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_carries_material_not_wording() {
        assert_eq!(
            RegisterScopeError::DuplicateScope {
                name: "feature".to_string()
            }
            .to_string(),
            "duplicate scope feature"
        );
        assert_eq!(
            RegisterScopeError::FreeformDefaultAlreadyTaken {
                holder: "express".to_string()
            }
            .to_string(),
            "freeform_default already taken by scope express"
        );
        assert_eq!(
            RegisterScopeError::UnknownStage {
                slug: StageSlug::parse("nope").expect("slug")
            }
            .to_string(),
            "unknown stage nope in scope column"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(RegisterScopeError::DuplicateScope {
            name: "x".to_string(),
        });
        assert_eq!(boxed.to_string(), "duplicate scope x");
    }
}
