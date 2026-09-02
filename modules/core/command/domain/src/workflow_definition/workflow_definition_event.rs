//! `WorkflowDefinitionEvent` — 定義集約に起きた事実。
//!
//! `WorkflowDefinition` は集約である (オーナー裁定 2026-08-29)。集約のファクトリは
//! **(集約インスタンス, 誕生イベント) の対を返す**ことが必須なので
//! (coding-rules/aggregate-commands.md)、定義側にもイベント語彙がある。
//!
//! # イベントは内容そのものを運ぶ (2026-08-31 改訂 — オーナー裁定「リポジトリの実装は
//! EventStoreForSqlite を使わないといけない」)
//!
//! かつてこのイベントは系譜 ID と内容版だけを運び、「内容の正本は実ファイル
//! (`stage-graph.json` / `scope-grid.json` / `scopes/*.md`) であり、それがこの集約の
//! リードモデルである」と述べていた。その前提は 2026-08-31 に破棄された — 集約の状態を
//! ファイルから組み立てる Repository は `coding-rules/cqrs-boundaries.md` 規則 4
//! (コマンド側の最新状態は常に集約から) への正面違反だからである。
//!
//! いまは**ジャーナルが内容の正本**である。したがってイベントは内容 (値) を運ぶ —
//! `Defined` から集約を導出する変換 [`From<(Defined, DateTime<Utc>)>`](WorkflowDefinition) がリプレイの
//! スナップショット種を与え、`Redefined` の差分適用が以後の内容を決める
//! (coding-rules/aggregate-commands.md「再構成の形」)。dist の 3 入力は**外部から来た
//! Published Language 成果物**であり、それを読んで定義を確立するのは書込ユースケースの
//! 取込境界の仕事になった。
//!
//! イベント 1 行が大きくなる (出荷定義は 33 ノード) のは ES の正常な代償である。再生の
//! コストはスナップショットが抑える — 再構成は最新のスナップショット行を基底に、その通番
//! より後の差分だけを畳む (ADR-010 / 本家 v3 の形)。
//!
//! [`WorkflowDefinition`]: super::workflow_definition::WorkflowDefinition

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — 利便再エクスポートではない。
// coding-rules/module-visibility.md)。
use super::workflow_definition_event_id::WorkflowDefinitionEventId;
use super::workflow_definition_id::WorkflowDefinitionId;

mod defined;
mod redefined;

pub use defined::Defined;
pub use redefined::Redefined;

/// 定義集約に起きた事実。
///
/// 誕生 (`Defined`) と改訂 (`Redefined`) の 2 変種。どちらも**その時点の内容を丸ごと**
/// 運ぶ — 定義の内容は 3 入力が lockstep で入れ替わる性質のもので、部分差分としての
/// 意味を持たないからである (compile は graph と grid を必ず一緒に出す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowDefinitionEvent {
    /// 定義が確立された (genesis)。
    Defined(Defined),
    /// 定義が別の内容版へ改訂された。
    Redefined(Redefined),
}

impl WorkflowDefinitionEvent {
    /// このイベント自身の識別子 (全変種が持つ — イベントはエンティティ)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionEventId {
        match self {
            WorkflowDefinitionEvent::Defined(payload) => payload.id(),
            WorkflowDefinitionEvent::Redefined(payload) => payload.id(),
        }
    }

    /// **どの集約の事実か** — 全変種が運ぶ系譜 ID。
    ///
    /// 復号境界 (Repository の再生・RMU の `decode_definition_row`) はこれと行の `aid` を
    /// 照合する。かつて `Redefined` が識別子を持たず「行の `aid` が正」としていた片肺は、
    /// 全変種が `aggregate_id` を持つことで解消した (オーナー裁定 2026-09-02)。
    #[must_use]
    pub const fn aggregate_id(&self) -> &WorkflowDefinitionId {
        match self {
            WorkflowDefinitionEvent::Defined(payload) => payload.aggregate_id(),
            WorkflowDefinitionEvent::Redefined(payload) => payload.aggregate_id(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// テスト用の固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
    fn definition_event_id() -> WorkflowDefinitionEventId {
        WorkflowDefinitionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003").unwrap()
    }
    use crate::workflow_definition::{
        DefinitionRevision, ExecutionKind, PhaseId, ScopeGrid, ScopeMetadata, StageGraph,
        StageMode, StageNodeBuilder, StageNumber, StageSlug, WorkflowDefinitionId,
    };

    fn id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("テストの定義 id")
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64)))
            .expect("テストの revision")
    }

    fn graph() -> StageGraph {
        StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug"),
                StageNumber::parse("0.1").expect("番号"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .build(),
        ])
        .expect("グラフ")
    }

    fn scopes() -> BTreeMap<String, ScopeMetadata> {
        [(
            "classic".to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect()
    }

    #[test]
    fn the_defined_payload_carries_the_lineage_the_revision_and_the_content() {
        // ジャーナルが内容の正本になったので、誕生イベントは内容そのものを運ぶ
        // (2026-08-31 — これが無いとジャーナルからの再構成が組めない)。
        let graph = graph();
        let grid = ScopeGrid::from_graph(&graph);
        let defined = Defined::new(
            definition_event_id(),
            id(),
            revision('0'),
            graph.clone(),
            grid.clone(),
            scopes(),
        );
        assert_eq!(defined.aggregate_id(), &id());
        assert_eq!(defined.id(), &definition_event_id());
        assert_eq!(defined.revision(), &revision('0'));
        assert_eq!(defined.graph(), &graph);
        assert_eq!(defined.grid(), &grid);
        assert_eq!(defined.scopes(), &scopes());
    }

    #[test]
    fn the_redefined_payload_carries_the_new_content_without_repeating_the_lineage() {
        // 系譜 ID はジャーナル行の集約識別子が持つ — 変異イベントは複製しない。
        let graph = graph();
        let grid = ScopeGrid::from_graph(&graph);
        let redefined = Redefined::new(
            definition_event_id(),
            id(),
            revision('1'),
            graph.clone(),
            grid.clone(),
            scopes(),
        );
        assert_eq!(redefined.revision(), &revision('1'));
        assert_eq!(redefined.graph(), &graph);
        assert_eq!(redefined.grid(), &grid);
        assert_eq!(redefined.scopes(), &scopes());
    }

    #[test]
    fn events_compare_by_value() {
        let graph = graph();
        let grid = ScopeGrid::from_graph(&graph);
        let first = WorkflowDefinitionEvent::Defined(Defined::new(
            definition_event_id(),
            id(),
            revision('0'),
            graph.clone(),
            grid.clone(),
            scopes(),
        ));
        let second = WorkflowDefinitionEvent::Defined(Defined::new(
            definition_event_id(),
            id(),
            revision('0'),
            graph.clone(),
            grid.clone(),
            scopes(),
        ));
        assert_eq!(first, second);
        let other = WorkflowDefinitionEvent::Defined(Defined::new(
            definition_event_id(),
            WorkflowDefinitionId::parse("kiro").expect("テストの定義 id"),
            revision('0'),
            graph.clone(),
            grid.clone(),
            scopes(),
        ));
        assert_ne!(first, other);
        // 誕生と改訂は同じ内容でも別の事実である。
        let redefined = WorkflowDefinitionEvent::Redefined(Redefined::new(
            definition_event_id(),
            id(),
            revision('0'),
            graph,
            grid,
            scopes(),
        ));
        assert_ne!(first, redefined);
    }

    #[test]
    fn both_variants_answer_their_own_id_and_the_lineage_id() {
        // b40 — 改訂も系譜 ID を運ぶので、どちらの変種でも照合の材料が揃う。
        let (graph, grid) = (graph(), ScopeGrid::from_graph(&graph()));
        for event in [
            WorkflowDefinitionEvent::Defined(Defined::new(
                definition_event_id(),
                id(),
                revision('0'),
                graph.clone(),
                grid.clone(),
                scopes(),
            )),
            WorkflowDefinitionEvent::Redefined(Redefined::new(
                definition_event_id(),
                id(),
                revision('1'),
                graph,
                grid,
                scopes(),
            )),
        ] {
            assert_eq!(event.id(), &definition_event_id());
            assert_eq!(event.aggregate_id(), &id());
        }
    }
}
