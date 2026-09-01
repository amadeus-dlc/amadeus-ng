//! `Parked` — `IntentExecutionEvent::Parked` のペイロード。

use crate::workflow_definition::StageSlug;

/// `Parked` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parked {
    stage: StageSlug,
}

impl Parked {
    /// park した位置のステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> Parked {
        Parked { stage }
    }

    /// park した位置のステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}
