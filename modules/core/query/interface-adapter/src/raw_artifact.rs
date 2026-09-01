//! `RawArtifact` — 生テキスト 1 つと、その出所 (逐語文言の材料)。

/// 生テキスト 1 つと、その出所 (逐語文言の材料)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawArtifact {
    pub(super) path: String,
    pub(super) text: String,
}

impl RawArtifact {
    /// 解決済みパスと読み終えた全文から組む。
    #[must_use]
    pub const fn new(path: String, text: String) -> RawArtifact {
        RawArtifact { path, text }
    }
}
