//! 公開記録のディレクトリ名と、衝突番号を含まない依頼のラベル。
use crate::workspace::IntentDirName;

/// 開始時に予約された公開記録名。内部のUUIDとは別の値である。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntentRecordName {
    directory: IntentDirName,
    slug: String,
}

impl IntentRecordName {
    /// 予約済みのディレクトリ名とラベルから組む。
    #[must_use]
    pub fn new(directory: IntentDirName, slug: impl Into<String>) -> Self {
        Self {
            directory,
            slug: slug.into(),
        }
    }
    /// 同日衝突の番号を含む公開ディレクトリ名。
    #[must_use]
    pub const fn directory(&self) -> &IntentDirName {
        &self.directory
    }
    /// 同日衝突の番号を含まないラベル。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }
}
