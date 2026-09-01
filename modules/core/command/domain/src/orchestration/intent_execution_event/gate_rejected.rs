//! `GateRejected` — `IntentExecutionEvent::GateRejected` のペイロード。

use crate::workflow_definition::StageSlug;

/// `GateRejected` のペイロード — 事実 (どのゲートが・どの理由で差し戻されたか) だけを運ぶ。
///
/// 改訂回数は載せない — 適用後の値 = 状態である。集約は自分のカウンタを +1 し、RMU は
/// リードモデルの `Revision Count` を read-modify-write する (upstream `aidlc-state.ts`
/// 自身が getField + 1 で書いており、この導出が正本互換 — オーナー裁定 2026-08-30)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRejected {
    stage: StageSlug,
    feedback: Option<String>,
}

impl GateRejected {
    /// 差し戻したステージと、差し戻し理由。
    #[must_use]
    pub const fn new(stage: StageSlug, feedback: Option<String>) -> GateRejected {
        GateRejected { stage, feedback }
    }

    /// 差し戻したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 差し戻し理由 (逐語保持)。
    #[must_use]
    pub fn feedback(&self) -> Option<&str> {
        self.feedback.as_deref()
    }
}
