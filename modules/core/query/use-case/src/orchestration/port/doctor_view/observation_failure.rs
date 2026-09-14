//! 観測が取れなかった事実 — 原因の材料だけを運ぶ (逐語文言は出す側が組む)。

use std::fmt;

/// ある観測元 (ファイル・環境・ストア) から事実を取り出せなかった。
///
/// 不在 (`missing`) は失敗ではなく各 View の `Option` / 変種が運ぶ。ここが運ぶのは
/// 「読もうとしたが読めなかった」「形が合わなかった」だけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationFailure {
    cause: String,
}

impl ObservationFailure {
    /// 原因の材料を束ねる (**唯一の構築経路**)。
    #[must_use]
    pub const fn new(cause: String) -> Self {
        Self { cause }
    }

    /// 原因の材料 (I/O の種類・パーサの拒否理由など)。
    #[must_use]
    pub fn cause(&self) -> &str {
        &self.cause
    }
}

impl fmt::Display for ObservationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.cause)
    }
}
