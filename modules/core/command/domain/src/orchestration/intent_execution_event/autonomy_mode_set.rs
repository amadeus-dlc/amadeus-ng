//! `AutonomyModeSet` — `IntentExecutionEvent::AutonomyModeSet` のペイロード。

use crate::orchestration::AutonomyMode;

/// `AutonomyModeSet` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyModeSet {
    mode: AutonomyMode,
}

impl AutonomyModeSet {
    /// 設定後のモード。
    #[must_use]
    pub const fn new(mode: AutonomyMode) -> AutonomyModeSet {
        AutonomyModeSet { mode }
    }

    /// 設定後のモード。
    #[must_use]
    pub const fn mode(&self) -> AutonomyMode {
        self.mode
    }
}
