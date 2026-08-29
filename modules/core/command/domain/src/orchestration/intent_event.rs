//! `IntentEvent` — intent 集約に起きた事実 (現在は genesis の 1 変種)。
//!
//! [`Intent`] は**集約**である (オーナー裁定 2026-08-30 — 静的で変異が現状無いだけで、
//! [`WorkflowDefinition`] と同じ類型)。集約のファクトリは **(集約インスタンス, 誕生イベント)
//! の対を返す**ことが必須なので (coding-rules/aggregate-commands.md)、intent 側にもイベント
//! 語彙を持たせる。
//!
//! 現スコープでは**ジャーナルへ接続しない** — 型と形だけを規則へ適合させ、`Created` を
//! `store` する `IntentRepository` は U7 (intent-create の実装) の課題である。
//!
//! [`Intent`]: super::intent::Intent
//! [`WorkflowDefinition`]: crate::workflow_definition::WorkflowDefinition

use serde::{Deserialize, Serialize};

use super::intent::Intent;

/// intent 集約に起きた事実。現在は genesis の 1 変種だけである。
///
/// 変異 (計画の再解決・依頼文の訂正など) が要件化したら、差分を運ぶ変種がここへ増える。
/// `#[non_exhaustive]` は付けない — 変種の追加は設計事項であり、消費側の網羅 match が
/// 落ちること自体が検出手段である ([`IntentExecutionEvent`] と同じ方針)。
///
/// [`IntentExecutionEvent`]: super::intent_execution_event::IntentExecutionEvent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentEvent {
    /// intent が作られた。
    Created(Created),
}

/// `Created` のペイロード — 作られた時点の intent を**丸ごと**運ぶ。
///
/// intent は静的 (Always Valid・変異メソッドなし) なので、**全属性がそのまま誕生の材料**で
/// ある。定義側の [`Defined`] が内容を焼かないのとは対照的だが、理由は明快で、定義には
/// 実ファイルというリードモデルが別にあるのに対し、intent の属性は intent 自身にしか無い。
///
/// イベントに集約の写しが載るのは**歴史の記録**であって埋め込みではない
/// (coding-rules/aggregate-references.md)。実行側の [`Started`] が同じ形を採っている。
///
/// [`Defined`]: crate::workflow_definition::Defined
/// [`Started`]: super::intent_execution_event::Started
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Created {
    intent: Intent,
}

impl Created {
    /// 作られた intent を束ねる。
    #[must_use]
    pub const fn new(intent: Intent) -> Created {
        Created { intent }
    }

    /// 作られた時点の intent (以後この写しは変わらない)。
    #[must_use]
    pub const fn intent(&self) -> &Intent {
        &self.intent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan};
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    fn intent() -> Intent {
        let stages = vec![StageEntry::new(
            StageSlug::parse("state-init").unwrap(),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            StageDisplay::new(
                StageNumber::parse("0.1").unwrap(),
                "State Init",
                "orchestrator",
            )
            .unwrap(),
        )];
        Intent::from_material(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build the thing"),
            stages,
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn the_created_payload_carries_the_whole_intent() {
        let created = Created::new(intent());
        assert_eq!(created.intent(), &intent());
        assert_eq!(created.intent().stage_count(), 1);
    }

    #[test]
    fn the_event_round_trips_through_serde() {
        let event = IntentEvent::Created(Created::new(intent()));
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<IntentEvent>(&json).unwrap(), event);
    }

    #[test]
    fn events_built_from_the_same_intent_compare_equal() {
        assert_eq!(
            IntentEvent::Created(Created::new(intent())),
            IntentEvent::Created(Created::new(intent()))
        );
    }
}
