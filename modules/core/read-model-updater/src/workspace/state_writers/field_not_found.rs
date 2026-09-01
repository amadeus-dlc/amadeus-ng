//! `state_writers::with_field` の拒否。

/// `with_field` の拒否 — 対象フィールド行が state ファイルに存在しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldNotFound {
    /// upstream 逐語の拒否文言 (文言カタログ経由)。
    message: String,
}

impl FieldNotFound {
    /// 文言カタログが組んだ拒否文言から構成する (文言の正本はカタログ側)。
    #[must_use]
    pub fn new(message: impl Into<String>) -> FieldNotFound {
        FieldNotFound {
            message: message.into(),
        }
    }

    /// upstream 逐語の拒否文言 — フィールド名を含む完成形で、Presenter はこれをそのまま出す。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
