//! `StageSlugView` — `/^[a-z][a-z0-9-]*$/`。ステージファイル名の stem と一致必須で、
//! コンパイル済みグラフ内で一意 (upstream 01 §3.2 / §8.4 #2,#3)。

use super::stage_slug_error::StageSlugError;

/// パース済みの stage slug (不正値はこの型に存在しない)。
///
/// `Ord` は生文字列の辞書順。数値順の語彙は [`super::StageNumberView`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageSlugView(String);

impl StageSlugView {
    /// # Errors
    ///
    /// 空・先頭非 `[a-z]`・`[a-z0-9-]` 以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<StageSlugView, StageSlugError> {
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
        Ok(StageSlugView(s.to_string()))
    }

    /// 生の slug 文字列 (正規化なし)。ステージファイル名の stem と一致する。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_shipping_vocabulary() {
        for s in ["intent-capture", "requirements-analysis", "s1", "a"] {
            assert_eq!(StageSlugView::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn rejects_shapes_that_could_not_be_a_stage_file_stem() {
        assert_eq!(StageSlugView::parse(""), Err(StageSlugError::Empty));
        assert_eq!(
            StageSlugView::parse("Intent"),
            Err(StageSlugError::InvalidLeading('I'))
        );
        assert_eq!(
            StageSlugView::parse("1stage"),
            Err(StageSlugError::InvalidLeading('1'))
        );
        assert_eq!(
            StageSlugView::parse("-stage"),
            Err(StageSlugError::InvalidLeading('-'))
        );
        assert_eq!(
            StageSlugView::parse("intent_capture"),
            Err(StageSlugError::InvalidChar('_'))
        );
        assert_eq!(
            StageSlugView::parse("Not A Slug"),
            Err(StageSlugError::InvalidLeading('N'))
        );
    }

    #[test]
    fn the_order_is_the_raw_lexicographic_one() {
        let a = StageSlugView::parse("a").unwrap();
        let b = StageSlugView::parse("b").unwrap();
        assert!(a < b);
    }
}
