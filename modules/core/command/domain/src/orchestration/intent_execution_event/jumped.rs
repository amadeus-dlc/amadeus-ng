//! `Jumped` — `IntentExecutionEvent::Jumped` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;

/// `Jumped` のペイロード — 到達点と、その時点のソース・成果物の外部観測を運ぶ。
///
/// 出発点・読み飛ばし列・巻き戻し列は載せない — すべて跳躍規則 (BR1.6) による導出で
/// あり、適用側 (集約) とリードモデル側 (RMU) がそれぞれ自分の状態 (カーソル・checkbox・
/// 実効プラン) から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumped {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    target: StageSlug,
    direction: crate::orchestration::JumpDirection,
    observation: Option<crate::orchestration::JumpObservation>,
    /// `--scope` で名指した別 scope の実効計画 (無ければこの実行の実効計画で導く)。
    scope: Option<crate::orchestration::JumpScope>,
}

impl Jumped {
    /// 跳んだ先。
    #[must_use]
    pub const fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        target: StageSlug,
        direction: crate::orchestration::JumpDirection,
        observation: Option<crate::orchestration::JumpObservation>,
    ) -> Jumped {
        Jumped {
            id,
            aggregate_id,
            target,
            direction,
            observation,
            scope: None,
        }
    }

    /// 直接 execute が `--scope` で名指した別 scope の実効計画を載せる (本家 `handleExecute`)。
    #[must_use]
    pub fn with_scope(mut self, scope: Option<crate::orchestration::JumpScope>) -> Jumped {
        self.scope = scope;
        self
    }

    /// 名指された別 scope。`None` ならこの実行の実効計画で到達可否と差分を導く。
    #[must_use]
    pub const fn scope(&self) -> Option<&crate::orchestration::JumpScope> {
        self.scope.as_ref()
    }

    /// executeで明示された方向。resolveの位置比較とは別の事実。
    #[must_use]
    pub const fn direction(&self) -> crate::orchestration::JumpDirection {
        self.direction
    }

    /// 実行時に採取した比較基準。再投影で現在のソースを読まない。
    #[must_use]
    pub fn baseline(&self) -> Option<&crate::orchestration::SourceBaseline> {
        self.observation
            .as_ref()
            .map(crate::orchestration::JumpObservation::baseline)
    }

    /// 永続化境界へ採取済みの観測を渡す。
    #[must_use]
    pub const fn observation(&self) -> Option<&crate::orchestration::JumpObservation> {
        self.observation.as_ref()
    }

    /// 跳んだ先。
    #[must_use]
    pub const fn target(&self) -> &StageSlug {
        &self.target
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
