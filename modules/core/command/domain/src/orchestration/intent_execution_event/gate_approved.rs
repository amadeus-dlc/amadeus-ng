//! `GateApproved` — `IntentExecutionEvent::GateApproved` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `GateApproved` のペイロード — 事実 (どのゲートが・どの入力で承認されたか) だけを運ぶ。
///
/// 次カーソルとフェーズ境界は載せない — どちらも導出された状態であり、適用側とリードモデル
/// 側が自分の状態から導く (オーナー裁定 2026-08-30)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateApproved {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    user_input: Option<String>,
}

impl GateApproved {
    /// 承認されたステージと、承認時の人間入力。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        user_input: Option<String>,
    ) -> GateApproved {
        GateApproved {
            id,
            aggregate_id,
            stage,
            user_input,
        }
    }

    /// 承認されたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 承認時の人間入力 (逐語保持)。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ (`coding-rules/domain-object-kinds.md`)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        &self.id
    }

    /// **どの集約の事実か** — この事実が起きた実行の識別子。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        &self.aggregate_id
    }
}
