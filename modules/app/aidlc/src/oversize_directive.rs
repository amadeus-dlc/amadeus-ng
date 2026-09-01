//! `Presenter::render` の自己防衛拒否 — 上限を超えた directive。

/// 上限を超えた directive — **emit 自体を拒否する**（half-emitted を出さない — I1）。
///
/// 材料だけを運ぶ（逐語文言は [`crate::wording`] が組む）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OversizeDirective {
    bytes: usize,
}

impl OversizeDirective {
    /// 描こうとした JSON のバイト数から組み立てる。
    ///
    /// `bytes` は private フィールドなので、`presenter` モジュールを跨いで構築するための
    /// 完全コンストラクタとして用意する（`coding-rules/field-visibility.md`
    /// 「跨ぐ必要があるなら、それは完全コンストラクタが無い合図」）。呼び手は
    /// `Presenter::render` のみ。
    #[must_use]
    pub(crate) const fn new(bytes: usize) -> OversizeDirective {
        OversizeDirective { bytes }
    }

    /// 描こうとした JSON のバイト数。
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }
}

impl core::fmt::Display for OversizeDirective {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "directive of {} bytes exceeds the cap", self.bytes)
    }
}

impl std::error::Error for OversizeDirective {}
