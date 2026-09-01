//! `StageRevised` — `IntentExecutionEvent::StageRevised` のペイロード。

use crate::workflow_definition::StageSlug;

/// `StageRevised` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRevised {
    stage: StageSlug,
}

impl StageRevised {
    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> StageRevised {
        StageRevised { stage }
    }

    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}
