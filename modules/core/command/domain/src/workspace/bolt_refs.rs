//! `BoltRefs` — 単一行リスト値。空は常に `[empty list]`、非空はソート済みブラケットリスト
//! (round-trip 決定的)。append/remove は重複・不在で **Err** (無言 no-op しない)
//! (upstream `aidlc-lib.ts:6635-6662`, 03 §5.2)。

use std::collections::BTreeSet;
use std::fmt;

/// 空リストの唯一の放出形。`parse` は `""` もこのリテラルも空として受理するが、`emit` は常に
/// こちらを書く (round-trip 決定性の要)。
pub const EMPTY_LIST_LITERAL: &str = "[empty list]";

/// `Bolt Refs` フィールドの値 — 進行中 Bolt の slug 集合 (fork で追加、merge で除去)。
/// 重複を持たず、放出順は入力順によらず整列。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoltRefs(BTreeSet<String>);

/// `BoltRefs` の拒否理由 (重複・不在を無言 no-op にしないための閉集合)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoltRefsError {
    /// ブラケットで始まらない・閉じない等の不正形。
    Malformed(String),
    /// append 対象が既に存在する。
    DuplicateSlug(String),
    /// remove 対象が存在しない。
    MissingSlug(String),
}

impl BoltRefs {
    /// 受理形: `""` / `[empty list]` / `[a, b]` (upstream `parseRefsList`)。
    /// # Errors
    ///
    /// ブラケット不整合・空要素は `Malformed`。
    pub fn parse(s: &str) -> Result<BoltRefs, BoltRefsError> {
        let t = s.trim();
        if t.is_empty() || t == EMPTY_LIST_LITERAL {
            return Ok(BoltRefs::default());
        }
        let inner = t
            .strip_prefix('[')
            .and_then(|x| x.strip_suffix(']'))
            .ok_or_else(|| BoltRefsError::Malformed(s.to_string()))?;
        let mut set = BTreeSet::new();
        for part in inner.split(',') {
            let slug = part.trim();
            if slug.is_empty() {
                return Err(BoltRefsError::Malformed(s.to_string()));
            }
            set.insert(slug.to_string());
        }
        Ok(BoltRefs(set))
    }

    /// 放出形は決定的: 空 → `[empty list]`、非空 → ソート済み `[a, b]` (upstream `emitRefsList`)。
    pub fn emit(&self) -> String {
        if self.0.is_empty() {
            EMPTY_LIST_LITERAL.to_string()
        } else {
            let joined: Vec<&str> = self.0.iter().map(String::as_str).collect();
            format!("[{}]", joined.join(", "))
        }
    }

    /// # Errors
    ///
    /// 重複 slug は `DuplicateSlug` (無言 no-op しない)。
    pub fn append_slug(&mut self, slug: &str) -> Result<(), BoltRefsError> {
        if !self.0.insert(slug.to_string()) {
            return Err(BoltRefsError::DuplicateSlug(slug.to_string()));
        }
        Ok(())
    }

    /// # Errors
    ///
    /// 不在 slug は `MissingSlug` (無言 no-op しない)。
    pub fn remove_slug(&mut self, slug: &str) -> Result<(), BoltRefsError> {
        if !self.0.remove(slug) {
            return Err(BoltRefsError::MissingSlug(slug.to_string()));
        }
        Ok(())
    }

    /// slug の在否 — `append_slug` / `remove_slug` が拒否するかの事前判定。
    #[must_use]
    pub fn contains(&self, slug: &str) -> bool {
        self.0.contains(slug)
    }

    /// 保持している slug の個数 (重複がないので集合の濃度そのもの)。
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 空か — 空は `emit` で `[empty list]` になる (空文字列は書かない)。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for BoltRefs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.emit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn removal_and_the_observation_faces_agree_with_the_set() {
        let mut refs = BoltRefs::parse("[b1-first, b2-second]").unwrap();
        assert!(refs.contains("b1-first"));
        assert!(!refs.contains("b9-absent"));
        assert_eq!(refs.len(), 2);
        assert!(!refs.is_empty());
        assert_eq!(
            refs.to_string(),
            refs.emit(),
            "Display は emit の綴りそのもの"
        );

        refs.remove_slug("b1-first").unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs.remove_slug("b1-first").unwrap_err(),
            BoltRefsError::MissingSlug("b1-first".to_string()),
            "不在 slug は無言 no-op にしない"
        );

        refs.remove_slug("b2-second").unwrap();
        assert!(refs.is_empty());
        assert_eq!(refs.emit(), "[empty list]");
    }

    #[test]
    fn empty_forms_parse_and_emit_the_literal() {
        assert_eq!(BoltRefs::parse("").unwrap().emit(), "[empty list]");
        assert_eq!(
            BoltRefs::parse("[empty list]").unwrap().emit(),
            "[empty list]"
        );
    }

    #[test]
    fn emit_is_sorted_regardless_of_input_order() {
        let refs = BoltRefs::parse("[zeta, alpha, mid]").unwrap();
        assert_eq!(refs.emit(), "[alpha, mid, zeta]");
    }

    #[test]
    fn append_and_remove_refuse_to_silently_no_op() {
        let mut refs = BoltRefs::parse("[a]").unwrap();
        assert_eq!(
            refs.append_slug("a"),
            Err(BoltRefsError::DuplicateSlug("a".into()))
        );
        assert_eq!(
            refs.remove_slug("b"),
            Err(BoltRefsError::MissingSlug("b".into()))
        );
        refs.append_slug("b").unwrap();
        refs.remove_slug("a").unwrap();
        assert_eq!(refs.emit(), "[b]");
    }

    #[test]
    fn malformed_lists_are_rejected() {
        assert!(BoltRefs::parse("a, b").is_err());
        assert!(BoltRefs::parse("[a, , b]").is_err());
        assert!(BoltRefs::parse("[a").is_err());
    }

    proptest! {
        /// emit → parse → emit の round-trip は不動点 (決定的直列化)。
        #[test]
        fn emit_parse_round_trip_is_a_fixed_point(
            slugs in proptest::collection::btree_set("[a-z][a-z0-9-]{0,8}", 0..6)
        ) {
            let mut refs = BoltRefs::default();
            for s in &slugs { refs.append_slug(s).unwrap(); }
            let emitted = refs.emit();
            let reparsed = BoltRefs::parse(&emitted).unwrap();
            prop_assert_eq!(reparsed.emit(), emitted);
        }
    }
}
