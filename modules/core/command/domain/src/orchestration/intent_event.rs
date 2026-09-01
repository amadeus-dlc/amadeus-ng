//! `IntentEvent` — intent 集約に起きた事実 (現在は genesis の 1 変種)。
//!
//! [`Intent`] は**集約**である (オーナー裁定 2026-08-30 — 静的で変異が現状無いだけで、
//! [`WorkflowDefinition`] と同じ類型)。集約のファクトリは **(集約インスタンス, 誕生イベント)
//! の対を返す**ことが必須なので (coding-rules/aggregate-commands.md)、intent 側にもイベント
//! 語彙を持たせる。
//!
//! `Created` は intent 自身のジャーナルへ書かれる — `store` / `find_by_id` を持つ
//! `IntentRepository` の実装はアダプタ層にある (issue #50)。ジャーナルへ書く upstream
//! コマンド (`intent-create`) の実装は U7 の課題である。
//!
//! イベントは**内容 (値) を運ぶ** — 集約はイベント列から `From<Created>` +
//! [`Intent::replay`] で導出する (オーナー裁定 2026-08-30、本家 v3 サンプル同型)。
//!
//! [`Intent`]: super::intent::Intent
//! [`Intent::replay`]: super::intent::Intent::replay
//! [`WorkflowDefinition`]: crate::workflow_definition::WorkflowDefinition

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — 利便再エクスポートではない。
// coding-rules/module-visibility.md)。
mod created;

pub use created::Created;

/// intent 集約に起きた事実。現在は genesis の 1 変種だけである。
///
/// 変異 (計画の再解決・依頼文の訂正など) が要件化したら、差分を運ぶ変種がここへ増える。
/// `#[non_exhaustive]` は付けない — 変種の追加は設計事項であり、消費側の網羅 match が
/// 落ちること自体が検出手段である ([`IntentExecutionEvent`] と同じ方針)。
///
/// [`IntentExecutionEvent`]: super::intent_execution_event::IntentExecutionEvent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentEvent {
    /// intent が作られた。
    Created(Created),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    use super::super::intent_id::IntentId;
    use super::super::stage_display::StageDisplay;
    use super::super::stage_entry::StageEntry;
    use super::super::start_request::StartRequest;
    use super::super::workspace_scan::WorkspaceScan;

    fn created() -> Created {
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
        Created::new(
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
    }

    #[test]
    fn the_created_payload_carries_the_birth_material() {
        let created = created();
        assert_eq!(
            created.id().to_string(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(created.definition_id().to_string(), "claude");
        assert_eq!(
            created.definition_revision().to_string(),
            format!("sha256:{}", "0".repeat(64))
        );
        assert_eq!(created.start_request().scope(), "classic");
        assert_eq!(created.stages().len(), 1);
        assert_eq!(created.scan().project_type(), "Greenfield");
    }

    #[test]
    fn events_built_from_the_same_material_compare_equal() {
        assert_eq!(
            IntentEvent::Created(created()),
            IntentEvent::Created(created())
        );
    }
}
