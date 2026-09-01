//! `StageSkipped` — `IntentExecutionEvent::StageSkipped` のペイロード。

use crate::workflow_definition::StageSlug;

/// `StageSkipped` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSkipped {
    stage: StageSlug,
    reason: String,
}

impl StageSkipped {
    /// 読み飛ばしたステージと、理由。次カーソルは載せない (導出 — オーナー裁定 2026-08-30)。
    #[must_use]
    pub const fn new(stage: StageSlug, reason: String) -> StageSkipped {
        StageSkipped { stage, reason }
    }

    /// 読み飛ばしたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 読み飛ばしの理由 (逐語保持)。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}
