//! `StageSlug` — `/^[a-z][a-z0-9-]*$/`。ステージファイル名の stem と一致必須で、
//! コンパイル済みグラフ内で一意 (upstream 01 §3.2 / §8.4 #2,#3)。

use std::fmt;

use super::stage_slug_error::StageSlugError;

/// パース済みの stage slug (Always Valid — 不正値はこの型に存在しない)。
///
/// `Ord` は生文字列の辞書順。数値順の語彙は `StageNumber` が持つ (両者を混同しないこと)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageSlug(String);

impl StageSlug {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<StageSlug, StageSlugError> {
        let mut chars = s.chars();
        match chars.next() {
            None => return Err(StageSlugError::Empty),
            Some(c) if !c.is_ascii_lowercase() => return Err(StageSlugError::InvalidLeading(c)),
            Some(_) => {}
        }
        for c in chars {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(StageSlugError::InvalidChar(c));
            }
        }
        Ok(StageSlug(s.to_string()))
    }

    /// 生の slug 文字列 (正規化なし)。ステージファイル名の stem と一致する。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for StageSlug {
    type Error = StageSlugError;

    fn try_from(value: String) -> Result<StageSlug, StageSlugError> {
        StageSlug::parse(&value)
    }
}

impl fmt::Display for StageSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(StageSlugError::Empty.to_string(), "empty");
        assert_eq!(
            StageSlugError::InvalidLeading('1').to_string(),
            "leading character '1'"
        );
        assert_eq!(
            StageSlugError::InvalidChar('_').to_string(),
            "invalid character '_'"
        );
    }

    #[test]
    fn accepts_the_shipping_vocabulary() {
        for s in [
            "intent-capture",
            "requirements-analysis",
            "code-generation",
            "s1",
            "a",
        ] {
            assert_eq!(StageSlug::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn rejects_shapes_that_could_not_be_a_stage_file_stem() {
        assert_eq!(StageSlug::parse(""), Err(StageSlugError::Empty));
        assert_eq!(
            StageSlug::parse("Intent"),
            Err(StageSlugError::InvalidLeading('I'))
        );
        assert_eq!(
            StageSlug::parse("1stage"),
            Err(StageSlugError::InvalidLeading('1'))
        );
        assert_eq!(
            StageSlug::parse("-stage"),
            Err(StageSlugError::InvalidLeading('-'))
        );
        assert_eq!(
            StageSlug::parse("intent_capture"),
            Err(StageSlugError::InvalidChar('_'))
        );
        assert_eq!(
            StageSlug::parse("a/b"),
            Err(StageSlugError::InvalidChar('/'))
        );
    }

    proptest! {
        /// E1: 文法に合う文字列は必ず受理され、`as_str` は逐語で返る。
        #[test]
        fn accepts_every_string_matching_the_grammar(s in "[a-z][a-z0-9-]{0,20}") {
            let slug = StageSlug::parse(&s).unwrap();
            prop_assert_eq!(slug.as_str(), s.as_str());
            prop_assert_eq!(slug.to_string(), s);
        }

        /// 文法外の文字を 1 つでも含めば拒否される。
        #[test]
        fn rejects_every_string_outside_the_grammar(
            head in "[a-z]",
            tail in "[a-z0-9-]{0,8}",
            bad in "[^a-z0-9-]",
        ) {
            let s = format!("{head}{tail}{bad}");
            prop_assert!(StageSlug::parse(&s).is_err());
        }
    }
}
