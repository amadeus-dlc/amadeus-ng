//! `DocumentInputBytes` — 封じ込めに成功した 1 ファイルの観測 (可搬パスと生バイト)。

/// 封じ込めに成功した 1 ファイルの観測。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInputBytes {
    path: String,
    bytes: Vec<u8>,
}

impl DocumentInputBytes {
    /// 可搬パスと生バイトを束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(path: String, bytes: Vec<u8>) -> DocumentInputBytes {
        DocumentInputBytes { path, bytes }
    }

    /// プロジェクトルート相対・前方スラッシュの可搬パス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 読み取った生バイト。
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
