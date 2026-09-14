//! この build 自身についての事実 — 合成ルートだけが知る配線表の照合結果。

use std::path::PathBuf;

/// 実行中バイナリ・配線されていない入口・フック面が受ける名前。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDoctorFacts {
    binary: Result<PathBuf, String>,
    missing_entry_points: Vec<String>,
    native_hook_names: Vec<String>,
}

impl NativeDoctorFacts {
    /// 事実を束ねる。
    #[must_use]
    pub const fn new(
        binary: Result<PathBuf, String>,
        missing_entry_points: Vec<String>,
        native_hook_names: Vec<String>,
    ) -> Self {
        Self {
            binary,
            missing_entry_points,
            native_hook_names,
        }
    }

    /// 実行中バイナリの所在 (取れなければ原因)。
    pub const fn binary(&self) -> &Result<PathBuf, String> {
        &self.binary
    }

    /// 配線表に無い入口。
    #[must_use]
    pub fn missing_entry_points(&self) -> &[String] {
        &self.missing_entry_points
    }

    /// `aidlc hook <name>` が受ける名前。
    #[must_use]
    pub fn native_hook_names(&self) -> &[String] {
        &self.native_hook_names
    }
}
