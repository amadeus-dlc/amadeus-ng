//! `CompiledDefinition` — コンパイル済み定義 (配布束) の集約。
//!
//! upstream の compile コンテキストが出力し、ハーネスと一緒に配られる 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md`) が表すもの。
//! **同一システム (AI-DLC v2 系) のドメインモデルの直列化形**であり、外部システムの
//! 成果物ではない (オーナー裁定 2026-09-02 — 「クライアントをリポジトリに、クライアントが
//! 扱うデータを集約に昇格」。#79 §1-4 / #80 の帰結)。
//!
//! # `WorkflowDefinition` との関係 — 別集約・同一系譜
//!
//! [`WorkflowDefinition`](super::WorkflowDefinition) は**ジャーナルに住む**定義
//! (define / redefine の履歴を持つ ES 集約)。`CompiledDefinition` は**配布された**定義
//! (compile の出力そのもの) である。両者は同じ系譜 ID (`harness.json` の `name` —
//! ADR-008) と内容版で結ばれ、`DefineWorkflowUseCase` が「配布された定義を読み、
//! ジャーナルの定義をそれに合わせる」— 集約 A を読んで集約 B を書く正規形
//! (`coding-rules/cqrs-boundaries.md` 規則 5) — の両端になる。
//!
//! # genesis はイベントの対を返す — 変異コマンドは未実装 (現スコープ)
//!
//! genesis ([`CompiledDefinition::compile`]) は (集約, [`Compiled`]) の対を返す —
//! Repository の `store(&event, &aggregate)` がジャーナル 1 行分とスナップショット分を
//! 対で受ける契約だからである (`coding-rules/aggregate-commands.md`)。変異コマンドは
//! まだ無い (`WorkflowDefinition` 先例と同じ位置づけ) — compile コンテキストが実装
//! されたら (slice 2)、再コンパイル等の状態遷移がイベントを吐く本則をそのまま適用する。

use std::collections::BTreeMap;

use super::compiled_definition_event::{Compiled, CompiledDefinitionEvent};
use super::compiled_definition_id::CompiledDefinitionId;
use super::definition_revision::DefinitionRevision;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;

/// コンパイル済み定義 (配布束) — 定義を確立・改訂するための内容の正本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledDefinition {
    id: CompiledDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl CompiledDefinition {
    /// 配布束を誕生させる (genesis — 対を返す)。
    ///
    /// 動詞 `compile` はこの集約を生む行為そのもの (upstream の compile / slice 2 の自前
    /// compile)。イベント [`Compiled`] は内容そのものを運ぶ ([`WorkflowDefinitionEvent::Defined`]
    /// と同じ理由)。再構成 ([`CompiledDefinition::new`]) はファクトリではないので委譲規律の
    /// 対象外である。
    ///
    /// [`WorkflowDefinitionEvent::Defined`]: super::WorkflowDefinitionEvent
    #[must_use]
    pub fn compile(
        id: CompiledDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> (CompiledDefinition, CompiledDefinitionEvent) {
        let compiled_definition = CompiledDefinition::new(
            id.clone(),
            revision.clone(),
            graph.clone(),
            grid.clone(),
            scopes.clone(),
        );
        (
            compiled_definition,
            CompiledDefinitionEvent::Compiled(Compiled::new(id, revision, graph, grid, scopes)),
        )
    }

    /// 検査済みの部品から再構成する (完全コンストラクタ — Repository の読取経路)。
    ///
    /// 部品は各 Domain Primitive が Always Valid を済ませている
    /// (`coding-rules/factory-naming.md`)。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> CompiledDefinition {
        CompiledDefinition {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 配布束が名乗る系譜 ID (`harness.json` の `name` — ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionId {
        &self.id
    }

    /// 読めた 3 入力の内容ダイジェスト。
    ///
    /// 「ディスクにあったバイトの版」ではなく「**読めた 3 入力の内容**の版」である —
    /// グリッドが欠けて転置導出へ倒れた場合も、導出結果を同じ形へ直列化して算出するので、
    /// 同じ内容の grid ファイルが置かれたときと同じ値になる。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド (欠損時は転置導出)。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// 内容を分解して手放す (`define` / `redefine` の引数へ渡すため)。
    ///
    /// 内容 3 点だけを返し、系譜 ID と内容版は返さない — 改訂は識別子を変えず、
    /// 内容版は呼出側が [`CompiledDefinition::revision`] で先に読むからである。
    /// これは境界の変換ではなく、集約 A の内容を集約 B のコマンド材料へ渡す分解である
    /// (イベントが材料の複製を運ぶのは歴史 — `coding-rules/aggregate-references.md`)。
    #[must_use]
    pub fn into_content(self) -> (StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>) {
        (self.graph, self.grid, self.scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNodeBuilder, StageNumber, StageSlug,
    };

    #[test]
    fn the_bundle_reports_back_every_part_it_was_given() {
        // 配布束が持つ 5 つの材料は、そのまま `define` / `redefine` の引数になる。
        // `into_content` は内容 3 点だけを手放し、系譜 ID と内容版は先に読む形である。
        let graph = StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug"),
                StageNumber::parse("0.1").expect("番号"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .scopes(vec!["classic".to_string()])
            .build(),
        ])
        .expect("グラフ");
        let grid = ScopeGrid::from_graph(&graph);
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect();
        let id = CompiledDefinitionId::parse("claude").expect("定義 id");
        let revision =
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision");

        let compiled_definition = CompiledDefinition::new(
            id.clone(),
            revision.clone(),
            graph.clone(),
            grid.clone(),
            scopes.clone(),
        );
        assert_eq!(compiled_definition.id(), &id);
        assert_eq!(compiled_definition.revision(), &revision);
        assert_eq!(compiled_definition.graph(), &graph);
        assert_eq!(compiled_definition.grid(), &grid);
        assert_eq!(compiled_definition.scopes(), &scopes);

        let (out_graph, out_grid, out_scopes) = compiled_definition.into_content();
        assert_eq!(out_graph, graph);
        assert_eq!(out_grid, grid);
        assert_eq!(out_scopes, scopes);
    }
}
