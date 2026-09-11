//! `ReviewedUnit` — 差し向け記録が名乗る、レビュー対象の Unit の綴り。
use super::ReviewedUnitError;
/// 差し向け記録 (`.aidlc-reviewer-dispatch.json`) の `unit` の**逐語**。
///
/// [`UnitName`] ではない — upstream `parseDispatchRecord` は `unit` が非空の文字列であること
/// しか見ず、`a/b` のような区切りを含む綴りもそのまま受けて読み取り範囲を組み、
/// `REVIEWER_SCOPE_BLOCKED` の `Unit` と拒否文言へ逐語で載せる (裁定 2026-09-10 Q1 = A)。
/// 空だけは Unit を名指していないので記録ごと読めない扱いになる。
///
/// [`UnitName`]: super::UnitName
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReviewedUnit(String);
impl ReviewedUnit {
    /// 記録の綴りを検査して持つ (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    /// 空の場合 (upstream `r.unit.length === 0` は記録全体を拒否する)。
    pub fn parse(raw: &str) -> Result<ReviewedUnit, ReviewedUnitError> {
        if raw.is_empty() {
            return Err(ReviewedUnitError::Empty);
        }
        Ok(ReviewedUnit(raw.to_string()))
    }
    /// 監査項目 `Unit`・拒否文言・読み取り範囲の照合に渡す逐語の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::{ReviewedUnit, ReviewedUnitError};
    #[test]
    fn a_unit_spelling_is_carried_verbatim_even_with_a_separator() {
        assert_eq!(
            ReviewedUnit::parse("u1-alpha").unwrap().as_str(),
            "u1-alpha"
        );
        assert_eq!(ReviewedUnit::parse("a/b").unwrap().as_str(), "a/b");
    }
    #[test]
    fn an_empty_spelling_names_no_unit() {
        assert_eq!(ReviewedUnit::parse(""), Err(ReviewedUnitError::Empty));
    }
}
