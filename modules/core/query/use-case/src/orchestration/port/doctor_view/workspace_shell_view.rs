//! D3.a の観測 — 配布シェルの配置。

/// ハーネス根と default space の memory 層の有無。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceShellView {
    harness_dir_exists: bool,
    memory_dir_exists: bool,
}

impl WorkspaceShellView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(harness_dir_exists: bool, memory_dir_exists: bool) -> Self {
        Self {
            harness_dir_exists,
            memory_dir_exists,
        }
    }

    /// `.claude/` が存在するか。
    #[must_use]
    pub const fn harness_dir_exists(&self) -> bool {
        self.harness_dir_exists
    }

    /// `aidlc/spaces/default/memory/` が存在するか。
    #[must_use]
    pub const fn memory_dir_exists(&self) -> bool {
        self.memory_dir_exists
    }
}
