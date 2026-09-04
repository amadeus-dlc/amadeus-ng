//! `PracticesAffirmed` — `IntentExecutionEvent::PracticesAffirmed` のペイロード。

use crate::orchestration::{IntentExecutionEventId, IntentExecutionId};
use crate::workflow_definition::StageSlug;
use crate::workspace::PromotedSection;

/// 承認された実践がメモリ層の正本へ**書き写された**事実（監査行 `PRACTICES_AFFIRMED`）。
///
/// 材料は昇格の内容そのものである — 置き換えた節 (`sections`) と、`## Mandated` /
/// `## Forbidden` へ足した**印付きの規則行**である。投影 (RMU) はこの材料だけで
/// team.md / project.md を描けるので、描く時点でドラフトを読み直す必要がない
/// （`coding-rules/aggregate-commands.md`「イベントが材料の複製を運ぶのは歴史である」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticesAffirmed {
    id: IntentExecutionEventId,
    aggregate_id: IntentExecutionId,
    stage: StageSlug,
    affirming_user: String,
    sections: Vec<PromotedSection>,
    mandated: Vec<String>,
    forbidden: Vec<String>,
}

impl PracticesAffirmed {
    /// 昇格の材料を束ねる。
    #[must_use]
    pub fn new(
        id: IntentExecutionEventId,
        aggregate_id: IntentExecutionId,
        stage: StageSlug,
        affirming_user: impl Into<String>,
        sections: Vec<PromotedSection>,
        mandated: Vec<String>,
        forbidden: Vec<String>,
    ) -> PracticesAffirmed {
        PracticesAffirmed {
            id,
            aggregate_id,
            stage,
            affirming_user: affirming_user.into(),
            sections,
            mandated,
            forbidden,
        }
    }

    /// 昇格が受領証を立てるステージ（常に practices-discovery）。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 昇格を打った人 — upstream の `--affirming-user`（既定は `unknown`）。
    #[must_use]
    pub fn affirming_user(&self) -> &str {
        &self.affirming_user
    }

    /// team.md で置き換える節（書込順）。
    #[must_use]
    pub fn sections(&self) -> &[PromotedSection] {
        &self.sections
    }

    /// `## Mandated` へ足す印付きの規則行。
    #[must_use]
    pub fn mandated(&self) -> &[String] {
        &self.mandated
    }

    /// `## Forbidden` へ足す印付きの規則行。
    #[must_use]
    pub fn forbidden(&self) -> &[String] {
        &self.forbidden
    }

    /// このイベント自身の識別子 — ドメインイベントはエンティティの一種なので自前の id を
    /// 持つ（`coding-rules/domain-object-kinds.md`）。
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
