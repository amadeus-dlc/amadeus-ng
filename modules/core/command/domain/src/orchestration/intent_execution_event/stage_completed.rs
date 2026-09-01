//! `StageCompleted` — `IntentExecutionEvent::StageCompleted` のペイロード。

use crate::workflow_definition::StageSlug;

/// `StageCompleted` のペイロード — 起きた事実 (どのステージが完了したか) だけを運ぶ。
///
/// 次カーソルは載せない — 導出された状態であり、適用側 (集約) とリードモデル側 (RMU) が
/// それぞれ自分の状態から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCompleted {
    stage: StageSlug,
}

impl StageCompleted {
    /// 完了したステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> StageCompleted {
        StageCompleted { stage }
    }

    /// 完了したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}
