//! `UnsplittableSection` — [`SteeringPlan::pack`] がセクションを輸送目標未満へ刻めなかった形。
//!
//! 防御的な拒否である (1 コードポイントが目標を超える場合にだけ成立する)。運ぶのは**材料
//! だけ** — どのファイルだったか — で、利用者向けの逐語文言は出す側が組む
//! (`coding-rules/error-handling.md`)。
//!
//! [`SteeringPlan::pack`]: super::SteeringPlan::pack

/// セクションを輸送目標未満へ分割できない (防御的 — 1 コードポイントが目標を超える場合のみ)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsplittableSection {
    path: String,
}

impl UnsplittableSection {
    /// 分割不能だったルールファイルのパスを包む。
    ///
    /// 構築するのは配信計画のパックだけなので `pub(super)` に留める。
    #[must_use]
    pub(super) const fn new(path: String) -> UnsplittableSection {
        UnsplittableSection { path }
    }

    /// 該当セクションを含むルールファイルのパス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl std::fmt::Display for UnsplittableSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsplittable section in {}", self.path)
    }
}

impl std::error::Error for UnsplittableSection {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unsplittable_error_names_its_file() {
        let error = UnsplittableSection {
            path: "org.md".to_string(),
        };
        assert_eq!(error.path(), "org.md");
        assert_eq!(error.to_string(), "unsplittable section in org.md");
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), "unsplittable section in org.md");
    }
}
