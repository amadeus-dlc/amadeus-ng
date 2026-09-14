//! `CodekbGeneration` — codekb ストアの世代 (compare-and-swap の片側)。

use std::fmt;

use super::codekb_token_error::CodekbTokenError;

/// ストアが 1 つも無いことを指す世代 (upstream 逐語)。
const ABSENT: &str = "none";

/// 在るストアの世代が名乗る前置き (upstream 逐語)。
const TREE_HASH_PREFIX: &str = "sha256:";

/// codekb ストアの世代。
///
/// **ストア全体**の内容から決まる値であり、鮮度印 1 枚ではない — 累積マージの最中に別の
/// 誰かがどれか 1 つの成果物を書き換えたら、この値が変わって古い公開が拒否される。
///
/// # 綴りを検査しない合言葉
///
/// [`CodekbGeneration::of_token`] は呼び手が `--expect-store` で渡した綴りをそのまま包む。
/// upstream は合言葉の形を検査せず突き合わせるだけなので、こちらが形を検査すると
/// upstream が `CODEKB_STORE_CHANGED` を答える場面で native だけ別の拒否を返してしまう。
/// 同値は綴りの一致で決める (`coding-rules/domain-equality.md`)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodekbGeneration(String);

impl CodekbGeneration {
    const fn of_value(value: String) -> Self {
        Self(value)
    }

    /// ストアが無いことを指す世代。
    #[must_use]
    pub fn absent() -> CodekbGeneration {
        Self::of_value(ABSENT.to_string())
    }

    /// 実測したストア木のハッシュから組む。
    #[must_use]
    pub fn of_tree_hash(hash: &str) -> CodekbGeneration {
        Self::of_value(format!("{TREE_HASH_PREFIX}{hash}"))
    }

    /// 呼び手が渡した合言葉を逐語で包む (`--expect-store`)。
    ///
    /// # Errors
    ///
    /// 空の合言葉を拒否する。
    pub fn of_token(raw: &str) -> Result<CodekbGeneration, CodekbTokenError> {
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

impl fmt::Display for CodekbGeneration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
