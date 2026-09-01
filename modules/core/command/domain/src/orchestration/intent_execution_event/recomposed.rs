//! `Recomposed` — `IntentExecutionEvent::Recomposed` のペイロード。

use crate::workflow_definition::StageSlug;

/// `Recomposed` のペイロード — 事実 (どの反転が起きたか) だけを運ぶ。
///
/// 適用後の in-scope 列は載せない — 適用後の状態であり、適用側とリードモデル側が自分の
/// 実効プランから導く (オーナー裁定 2026-08-30)。1 コマンドの複数反転は 1 イベント (C5)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomposed {
    skipped: Vec<StageSlug>,
    added: Vec<StageSlug>,
}

impl Recomposed {
    /// EXECUTE → SKIP にした列、SKIP → EXECUTE にした列。
    #[must_use]
    pub const fn new(skipped: Vec<StageSlug>, added: Vec<StageSlug>) -> Recomposed {
        Recomposed { skipped, added }
    }

    /// EXECUTE → SKIP に反転したステージ列。
    #[must_use]
    pub fn skipped(&self) -> &[StageSlug] {
        &self.skipped
    }

    /// SKIP → EXECUTE に反転したステージ列。
    #[must_use]
    pub fn added(&self) -> &[StageSlug] {
        &self.added
    }
}
