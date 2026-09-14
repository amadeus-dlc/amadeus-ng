//! D2.f の観測 — hook heartbeat と、同じ記録の進行の証拠。

use super::{HeartbeatEntry, ObservedTimestamp};

/// `.aidlc-hooks-health/` の状態と、状態ファイル・監査シャードが示す進行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatObservation {
    health_dir_exists: bool,
    has_heartbeat_files: bool,
    entries: Vec<HeartbeatEntry>,
    progressed_stage_count: usize,
    stage_started: bool,
    newest_progress: Option<ObservedTimestamp>,
}

impl HeartbeatObservation {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        health_dir_exists: bool,
        has_heartbeat_files: bool,
        entries: Vec<HeartbeatEntry>,
        progressed_stage_count: usize,
        stage_started: bool,
        newest_progress: Option<ObservedTimestamp>,
    ) -> Self {
        Self {
            health_dir_exists,
            has_heartbeat_files,
            entries,
            progressed_stage_count,
            stage_started,
            newest_progress,
        }
    }

    /// health ディレクトリが存在するか。
    #[must_use]
    pub const fn health_dir_exists(&self) -> bool {
        self.health_dir_exists
    }

    /// `.last` ファイルが 1 つでもあるか (読めたかは別)。
    #[must_use]
    pub const fn has_heartbeat_files(&self) -> bool {
        self.has_heartbeat_files
    }

    /// 読めた heartbeat (ファイル列挙順)。
    #[must_use]
    pub fn entries(&self) -> &[HeartbeatEntry] {
        &self.entries
    }

    /// 進行済みステージ数 (状態ファイルの非 pending 行と監査の STAGE_/GATE_ の多いほう)。
    #[must_use]
    pub const fn progressed_stage_count(&self) -> usize {
        self.progressed_stage_count
    }

    /// 監査に `STAGE_STARTED` があるか。
    #[must_use]
    pub const fn stage_started(&self) -> bool {
        self.stage_started
    }

    /// 監査の STAGE_/GATE_ 行のうち最新の時刻。
    #[must_use]
    pub const fn newest_progress(&self) -> Option<&ObservedTimestamp> {
        self.newest_progress.as_ref()
    }
}
