//! `WorkspaceLayout` — レコードとステージ資産の配置。
//!
//! パス組み立ての材料であり、観測そのものは Controller (U7) が供給する。ここは
//! 「3 つの配置が揃っている」ことだけを型で保証する。

/// レコードとステージ資産の配置 (パス組み立ての材料 — Controller 供給)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    record_dir: String,
    stage_library_dir: String,
    agent_dir: String,
}

impl WorkspaceLayout {
    /// 3 つの配置を束ねる。
    #[must_use]
    pub const fn new(
        record_dir: String,
        stage_library_dir: String,
        agent_dir: String,
    ) -> WorkspaceLayout {
        WorkspaceLayout {
            record_dir,
            stage_library_dir,
            agent_dir,
        }
    }

    /// 稼働中 intent の record ディレクトリ (`aidlc/spaces/<space>/intents/<slug>-<id8>`)。
    #[must_use]
    pub fn record_dir(&self) -> &str {
        &self.record_dir
    }

    /// ステージ本体ファイルの置き場 (`.claude/aidlc-common/stages`)。
    #[must_use]
    pub fn stage_library_dir(&self) -> &str {
        &self.stage_library_dir
    }

    /// エージェントペルソナの置き場 (`.claude/agents`)。
    #[must_use]
    pub fn agent_dir(&self) -> &str {
        &self.agent_dir
    }
}
