//! `settings.json` が名指す `aidlc-*.ts` 1 本の存在観測。

/// 配線されたフック名と、`.claude/hooks/` にその実体があるか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiredHook {
    name: String,
    present: bool,
}

impl WiredHook {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(name: String, present: bool) -> Self {
        Self { name, present }
    }

    /// ファイル名 (`aidlc-write-audit-log.ts` 等)。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `.claude/hooks/<name>` が存在するか。
    #[must_use]
    pub const fn present(&self) -> bool {
        self.present
    }
}
