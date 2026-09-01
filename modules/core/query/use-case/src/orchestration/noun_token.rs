//! `NounToken` — 名詞トークン (族 + 残りのトークン列)。
//!
//! 族 ([`NounFamily`]) は先頭トークンだけが決め、残りのトークン列は**逐語で通す** —
//! 人間の語をそのままエンジンコマンドへ運ぶための材料である。

use super::noun_family::NounFamily;

/// 名詞トークン (族 + 残りのトークン列は逐語で通す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NounToken {
    family: NounFamily,
    tokens: Vec<String>,
}

impl NounToken {
    /// 族と残トークンを束ねる。
    #[must_use]
    pub const fn new(family: NounFamily, tokens: Vec<String>) -> NounToken {
        NounToken { family, tokens }
    }

    /// 族。
    #[must_use]
    pub const fn family(&self) -> NounFamily {
        self.family
    }

    /// 先頭トークンを含む残りのトークン列 (逐語)。
    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}
