//! `PlanError` — 解決済み計画そのものの不変条件を破った形。

use std::fmt;

/// 解決済み計画 (文書順の [`StageEntry`] 列) が満たすべき不変条件の違反
/// (材料のみ — 利用者向け文言はアダプタ層)。
///
/// 検査の正本は [`StageEntry::check_plan`] であり、intent の鋳造 ([`Intent::create`]) も
/// 実行の誕生記録の復号 (両側の `StartedDto`) も同じ 1 か所を通る。initialization
/// フェーズの扱いは BR2.2 — 状態ファイルを起こす工程そのものなので、SKIP にも
/// CONDITIONAL にもできない。
///
/// [`StageEntry`]: crate::orchestration::StageEntry
/// [`StageEntry::check_plan`]: crate::orchestration::StageEntry::check_plan
/// [`Intent::create`]: crate::orchestration::Intent::create
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// 解決済み計画が 0 件。
    Empty,
    /// initialization フェーズのステージが SKIP に畳まれた、または先頭ステージが SKIP。
    InitializationMustExecute,
    /// initialization フェーズのステージが CONDITIONAL。
    InitializationMustBeUnconditional,
    /// 同じ slug が 2 回以上現れる (BR1.5 — ステージ参照の解決先が一意でなくなる)。
    DuplicateSlug {
        /// 文書順で最初に重複した slug。
        slug: String,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::Empty => f.write_str("empty stage list"),
            PlanError::InitializationMustExecute => {
                f.write_str("initialization stage is not EXECUTE")
            }
            PlanError::InitializationMustBeUnconditional => {
                f.write_str("initialization stage is CONDITIONAL")
            }
            PlanError::DuplicateSlug { slug } => write!(f, "duplicate stage slug: {slug}"),
        }
    }
}

impl std::error::Error for PlanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_violation_renders_its_material() {
        assert_eq!(PlanError::Empty.to_string(), "empty stage list");
        assert_eq!(
            PlanError::InitializationMustExecute.to_string(),
            "initialization stage is not EXECUTE"
        );
        assert_eq!(
            PlanError::InitializationMustBeUnconditional.to_string(),
            "initialization stage is CONDITIONAL"
        );
        assert_eq!(
            PlanError::DuplicateSlug {
                slug: "intent-capture".to_string(),
            }
            .to_string(),
            "duplicate stage slug: intent-capture"
        );
    }

    #[test]
    fn the_violation_is_a_std_error() {
        let error: &dyn std::error::Error = &PlanError::Empty;
        assert!(error.source().is_none(), "材料を自分で持つので連鎖しない");
    }

    #[test]
    fn violations_compare_by_value() {
        assert_eq!(PlanError::Empty, PlanError::Empty);
        assert_ne!(PlanError::Empty, PlanError::InitializationMustExecute);
    }
}
