//! `RuleContent` — 参照入力から読んだ規則ファイルの 1 片 (パスと本文)。

/// 規則ファイルの 1 片 — パスと本文の対。
///
/// パックの前は「読んだファイル 1 本」、パックの後は「1 チャンクに入る断片」を表す。
/// どちらも同じ形で足りるのは、分割が本文を切るだけでパスを変えないからである。
///
/// 本文が必須 steering であり、パスはルーティングのメタデータである (02 §10「No rule is
/// downgraded to a discretionary path read」)。クエリ側の同名型の**写し**であり、Bolt 3 で
/// クエリ側の複製が消える (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleContent {
    path: String,
    text: String,
}

impl RuleContent {
    /// パスと本文を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(path: String, text: String) -> RuleContent {
        RuleContent { path, text }
    }

    /// 規則ファイルのパス (読み手が決めた綴りをそのまま運ぶ)。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 規則の本文 (必須 steering — 正規化も切り詰めもしない)。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_carries_its_path_and_its_body_verbatim() {
        let piece = RuleContent::new("org.md".to_string(), "# Org\r\n".to_string());
        assert_eq!(piece.path(), "org.md");
        assert_eq!(piece.text(), "# Org\r\n", "本文は 1 バイトも変えない");
        assert_eq!(piece.clone(), piece);
    }
}
