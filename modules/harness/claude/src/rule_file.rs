//! `RuleFile` — 規則配送で 1 件ぶんの「配送する綴り」と「本文」。

/// 束に載る規則ファイル 1 件。
///
/// 本家 `tools/aidlc-steering.ts` の `RuleContent = { path: string; text: string }` に
/// 対応する。`path` は**配送する綴り**（`aidlc/spaces/<space>/memory/<subpath>` の POSIX
/// 形）であって読取先の実パスではない — 束のダイジェストがこの綴りをそのまま素材にする
/// ので、読取先を差し替えても綴りは変わらない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFile {
    path: String,
    text: String,
}

impl RuleFile {
    /// 綴りと本文から 1 件を組む（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(path: String, text: String) -> RuleFile {
        RuleFile { path, text }
    }

    /// 配送する綴り。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 規則の本文（逐語）。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}
