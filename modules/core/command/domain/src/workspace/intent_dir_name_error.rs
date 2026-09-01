//! `IntentDirNameError` — `IntentDirName::parse` の拒否理由。

use std::fmt;

use super::intent_dir_name::MAX_LEN;

/// `IntentDirName::parse` が拒否する形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentDirNameError {
    /// 空文字列。
    Empty,
    /// 64 字を超える。
    Length {
        /// 実際の文字数。
        actual: usize,
    },
    /// 日付プレフィクスの数字・区切り・slug の `[a-z0-9-]` の並びに合わない文字がある。
    Format {
        /// 最初に形式へ合わなかった文字の 0 始まり位置 (末端で尽きた場合はその位置)。
        position: usize,
    },
    /// `-` で区切った区間が空 (`--` の連続、末尾の `-`、slug 不在)。
    EmptySegment {
        /// 空だった区間の 0 始まり位置 (区間 0 は日付プレフィクス)。
        position: usize,
    },
}

impl fmt::Display for IntentDirNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentDirNameError::Empty => f.write_str("empty"),
            IntentDirNameError::Length { actual } => {
                write!(f, "length {actual} (maximum {MAX_LEN})")
            }
            IntentDirNameError::Format { position } => {
                write!(f, "invalid character at position {position}")
            }
            IntentDirNameError::EmptySegment { position } => {
                write!(f, "empty segment at position {position}")
            }
        }
    }
}

impl std::error::Error for IntentDirNameError {}
