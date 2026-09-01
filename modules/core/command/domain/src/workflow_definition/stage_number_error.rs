//! `StageNumberError` — `StageNumber::parse` が拒否する形。

use std::fmt;

/// `StageNumber::parse` が拒否する形の違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageNumberError {
    /// 入力が空文字列。
    Empty,
    /// `.` がちょうど 1 個でない。
    MalformedSeparator {
        /// 入力に含まれる `.` の個数 (期待値は 1)。
        dot_count: usize,
    },
    /// `.` の左側 (`<phaseIndex>`) が空。
    EmptyPhaseIndex,
    /// `.` の右側 (`<seq>`) が空。
    EmptySeq,
    /// ASCII 数字以外を含む (符号・空白を含む)。
    NonDigit(char),
    /// `u32` に収まらない。
    Overflow,
}

impl fmt::Display for StageNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 材料のみ (利用者向け文言はアダプタ層)。
        match self {
            StageNumberError::Empty => f.write_str("stage number is empty"),
            StageNumberError::MalformedSeparator { dot_count } => {
                write!(f, "stage number has {dot_count} dots (expected 1)")
            }
            StageNumberError::EmptyPhaseIndex => f.write_str("stage number has an empty phase"),
            StageNumberError::EmptySeq => f.write_str("stage number has an empty sequence"),
            StageNumberError::NonDigit(found) => {
                write!(f, "stage number has a non-digit: {found:?}")
            }
            StageNumberError::Overflow => f.write_str("stage number does not fit in u32"),
        }
    }
}

impl std::error::Error for StageNumberError {}
