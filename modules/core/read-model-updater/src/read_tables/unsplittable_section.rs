//! `UnsplittableSection` — セクションを輸送目標未満へ刻めなかった形 (防御的)。

/// セクションを輸送目標未満へ分割できない (1 コードポイントが目標を超える場合のみ成立)。
///
/// 運ぶのは**材料だけ** — どのファイルだったか — であり、利用者向けの逐語文言は出す側が
/// 組む (`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsplittableSection {
    path: String,
}

impl UnsplittableSection {
    /// 分割不能だった規則ファイルのパスを包む (**この型の唯一の構築経路**)。
    #[must_use]
    pub(crate) const fn new(path: String) -> UnsplittableSection {
        UnsplittableSection { path }
    }

    /// 該当セクションを含む規則ファイルのパス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl core::fmt::Display for UnsplittableSection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unsplittable section in {}", self.path)
    }
}

impl std::error::Error for UnsplittableSection {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_names_the_file_it_could_not_split() {
        let error = UnsplittableSection::new("memory/org.md".to_string());
        assert_eq!(error.path(), "memory/org.md");
        assert_eq!(error.to_string(), "unsplittable section in memory/org.md");
        assert!(std::error::Error::source(&error).is_none());
    }
}
