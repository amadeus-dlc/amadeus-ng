//! `CodekbSourceFingerprint` — 走査が読んだ源の指紋 (compare-and-swap のもう片側)。

use std::fmt;

use super::codekb_token_error::CodekbTokenError;

/// git の作業ツリーから採れた指紋の前置き (upstream 逐語)。
const GIT_PREFIX: &str = "git:";

/// git が使えないときに木のハッシュで採った指紋の前置き (upstream 逐語)。
const TREE_PREFIX: &str = "tree:";

/// 走査が読んだ源の指紋。
///
/// **由来を綴りに含める**のが要点である (`git:` / `tree:`) — git の内容アドレスと、こちらで
/// 畳んだ木のハッシュは別の空間の値なので、偶然の一致で「変わっていない」と読ませない。
///
/// 合言葉としての振る舞いは [`super::CodekbGeneration`] と同じで、呼び手が
/// `--expect-source` で渡した綴りは検査せずに逐語で運ぶ。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodekbSourceFingerprint(String);

impl CodekbSourceFingerprint {
    const fn of_value(value: String) -> Self {
        Self(value)
    }

    /// git の作業ツリーから採れた指紋。
    #[must_use]
    pub fn of_git(hash: &str) -> CodekbSourceFingerprint {
        Self::of_value(format!("{GIT_PREFIX}{hash}"))
    }

    /// git が使えないときの、木のハッシュによる指紋。
    #[must_use]
    pub fn of_tree(hash: &str) -> CodekbSourceFingerprint {
        Self::of_value(format!("{TREE_PREFIX}{hash}"))
    }

    /// 呼び手が渡した合言葉を逐語で包む (`--expect-source`)。
    ///
    /// # Errors
    ///
    /// 空の合言葉を拒否する。
    pub fn of_token(raw: &str) -> Result<CodekbSourceFingerprint, CodekbTokenError> {
        if raw.is_empty() {
            return Err(CodekbTokenError::Empty);
        }
        Ok(Self::of_value(raw.to_string()))
    }

    /// 出力・突合へ渡す綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodekbSourceFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
