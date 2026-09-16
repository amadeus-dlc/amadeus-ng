//! 直接入力を受理できなかった — 材料だけを運ぶ拒否。
//!
//! 「引けなかった」だけを語る [`DocumentInputReadError`] と違い、こちらは**引けたバイトが
//! 直接入力の条件を満たさない**ことも語る (1 行でない、UTF-8 でない、種別が違う、上限を
//! 超えている)。どちらも運ぶのは材料だけで、利用者向けの逐語文言は出す側 (プレゼンタ) が
//! 組む (`coding-rules/error-handling.md`)。

use core::fmt;
use std::error::Error;

use crate::orchestration::DocumentInputReadError;

/// 直接入力を受理できなかった。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentInputError {
    /// 転送ファイルを引けなかった。
    RequestUnreadable(DocumentInputReadError),
    /// 転送ファイルが 1 本の非空パス行になっていない (空・複数行)。
    RequestNotOneLine,
    /// 転送ファイルが UTF-8 として読めない。
    RequestNotUtf8,
    /// 名指された 1 ファイルを引けなかった (プロジェクト外・不在・非通常ファイルなど)。
    DocumentUnreadable(DocumentInputReadError),
    /// 直接扱える種別ではない (材料 = 可搬パスと、判定した種別)。
    UnsupportedType {
        /// 可搬パス。
        path: String,
        /// 判定した種別。
        media_type: String,
    },
    /// 直接入力の文字数上限を超えている (材料 = 可搬パス・実文字数・上限)。
    TooManyCharacters {
        /// 可搬パス。
        path: String,
        /// 実文字数。
        characters: usize,
        /// 上限。
        cap: usize,
    },
}

impl fmt::Display for DocumentInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestUnreadable(cause) => write!(formatter, "request unreadable: {cause}"),
            Self::RequestNotOneLine => write!(formatter, "request is not one non-empty path line"),
            Self::RequestNotUtf8 => write!(formatter, "request is not UTF-8"),
            Self::DocumentUnreadable(cause) => write!(formatter, "document unreadable: {cause}"),
            Self::UnsupportedType { path, media_type } => {
                write!(formatter, "{path} is {media_type}")
            }
            Self::TooManyCharacters {
                path,
                characters,
                cap,
            } => write!(formatter, "{path} has {characters} characters (cap {cap})"),
        }
    }
}

impl Error for DocumentInputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RequestUnreadable(cause) | Self::DocumentUnreadable(cause) => Some(cause),
            _ => None,
        }
    }
}
