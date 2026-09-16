//! `DocumentInputView` — 直接入力として受理した 1 ファイルの観測。

/// 受理した直接入力 — 可搬パス、バイト数、UTF-8 本文。
///
/// 信頼注意書きはこの View に含めない。注意書きは**出す側が必ず同じ JSON へ添える**固定の
/// 逐語であり、引いた結果ではないからである (`coding-rules/cqrs-boundaries.md` 規則 7 —
/// 文言はプレゼンタが組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInputView {
    path: String,
    bytes: usize,
    content: String,
}

impl DocumentInputView {
    /// 受理した観測を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(path: String, bytes: usize, content: String) -> DocumentInputView {
        DocumentInputView {
            path,
            bytes,
            content,
        }
    }

    /// プロジェクトルート相対・前方スラッシュの可搬パス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 読み取った生バイト数。
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// UTF-8 として解釈した本文。
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}
