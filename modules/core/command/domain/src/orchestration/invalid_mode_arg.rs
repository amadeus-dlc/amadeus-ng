//! `InvalidModeArg` — CLI `--mode` の 2 値厳密パースの拒否が運ぶ材料。

/// CLI `--mode` の 2 値厳密パースの拒否 — **材料のみ**（与えられた不正値）を運ぶ。
/// 利用者向け文言はアダプタ層が組む (2026-08-29 是正 — 従来は完成文言を運んでいた)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidModeArg {
    /// 与えられた不正な `--mode` 値（そのまま）。
    given: String,
}

impl InvalidModeArg {
    /// 拒否された値を包む（整形も加工もしない — 文言はここの責務ではない）。
    #[must_use]
    pub fn new(given: impl Into<String>) -> InvalidModeArg {
        InvalidModeArg {
            given: given.into(),
        }
    }

    /// 与えられた不正な `--mode` 値。
    #[must_use]
    pub fn given(&self) -> &str {
        &self.given
    }
}
