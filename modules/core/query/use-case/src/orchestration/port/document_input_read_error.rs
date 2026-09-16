//! `DocumentInputReadError` — 直接入力の 1 面を引けなかったことを、材料だけで語る拒否。
//!
//! 「プロジェクトルートの外を指した」と「直接読めなかった」は、利用者へ返す remedy が別なので
//! 変種を分ける。逐語文言は出す側 (プレゼンタ) が組む。

use std::error::Error;
use std::fmt;

/// 直接入力の 1 ファイルを引けなかった (材料のみ — 逐語は出す側が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentInputReadError {
    /// 解決先がプロジェクトルートの外にある (検索も fallback もしない)。
    OutsideProject {
        /// 顧客が転送ファイルへ書いた綴りそのもの。
        requested: String,
    },
    /// 直接読めない — 不在・シンボリックリンク・非通常ファイル・上限超過・読取中の変化。
    Unreadable {
        /// 読もうとした対象の呼び名 (転送ファイル名、または可搬パス)。
        what: String,
        /// 媒体が語った原因。
        cause: String,
    },
}

impl fmt::Display for DocumentInputReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideProject { requested } => {
                write!(formatter, "{requested} resolves outside the project root")
            }
            Self::Unreadable { what, cause } => write!(formatter, "{what}: {cause}"),
        }
    }
}

impl Error for DocumentInputReadError {}
