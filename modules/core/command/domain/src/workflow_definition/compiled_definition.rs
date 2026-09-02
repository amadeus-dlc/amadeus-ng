//! `CompiledDefinition` — コンパイル済み定義 (配布束) の集約。
//!
//! upstream の compile コンテキストが出力し、ハーネスと一緒に配られる 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md`) が表すもの。
//! **同一システム (AI-DLC v2 系) のドメインモデルの直列化形**であり、外部システムの
//! 成果物ではない (オーナー裁定 2026-09-02 — 「クライアントをリポジトリに、クライアントが
//! 扱うデータを集約に昇格」。#79 §1-4 / #80 の帰結)。
//!
//! # FSM — 状態 = 内容 + 内容版、遷移 = 配布束そのものが変わる出来事
//!
//! 配布束は「一度作られたら二度と変わらない値」ではなく、識別子 (系譜名) を保ったまま
//! 内容が変わっていくライフサイクルを持つ (オーナー裁定 2026-09-02、b36)。状態ラベルは
//! 持たず、遷移のたびに内容が変わり内容版 [`DefinitionRevision`] が付け替わる型の FSM
//! である ([`WorkflowDefinition`](super::WorkflowDefinition) と同型)。
//!
//! | 遷移 | イベント | ガード (`Err`) | 起こす人 |
//! |---|---|---|---|
//! | [`compile`](CompiledDefinition::compile) (genesis) | `Compiled` | — | compile コンテキスト |
//! | [`recompile`](CompiledDefinition::recompile) | `Recompiled` | 内容が同じ (`Unchanged`) | compile コンテキスト (源が変わったとき) |
//! | [`register_scope`](CompiledDefinition::register_scope) | `ScopeRegistered` | 名前重複 / `freeform_default` 二重 / グラフに無い slug | コンポーザ承認 |
//! | [`apply_plugin_selection`](CompiledDefinition::apply_plugin_selection) | `PluginSelectionApplied` | 未知プラグイン / 変化なし | `select-plugins` |
//!
//! 「源が変わったのに再コンパイルされていない (stale)」は配布束の状態ではなく源との
//! 関係なので、ここには無い — 照合は compile コンテキストの問い合わせ (`compile --check`)
//! であり、配布束は `recompile` の `Unchanged` ガードで「変わっていない」と答えるだけである。
//!
//! # 内容版は自分で導出する (ADR-008 改訂 2026-09-02)
//!
//! 内容版は内容の純粋な関数 ([`DefinitionRevision::of_content`]) なので、集約が遷移のたびに
//! 再計算する。呼出側に「適用後の内容を先読みして計算させる」ことはしない
//! (tell-dont-ask)。永続化面 (配布 3 ファイル) には内容版を書かない — 読み戻せば同じ値が
//! 導出される。
//!
//! # `WorkflowDefinition` との関係 — 別集約・同一系譜
//!
//! `WorkflowDefinition` は**ジャーナルに住む**定義 (define / redefine の履歴を持つ ES 集約)、
//! `CompiledDefinition` は**配布された**定義である。両者は同じ系譜 ID (`harness.json` の
//! `name` — ADR-008) で結ばれ、`DefineWorkflowUseCase` が「配布された定義を読み、ジャーナルの
//! 定義をそれに合わせる」— 集約 A を読んで集約 B を書く正規形 (`coding-rules/cqrs-boundaries.md`
//! 規則 5) — の両端になる。系譜の照合は受け手 (`WorkflowDefinition::define` / `redefine`)
//! がガードする。
//!
//! # 構築口は genesis と `From<Compiled>` だけ
//!
//! 永続化面はジャーナルではなくスナップショット (配布ファイル) なので `replay` は無い。
//! 読取は媒体から復号した内容を [`Compiled`] に束ね、genesis と同じ変換で集約を起こす。
//! decide / apply は分離し、コマンドは [`apply_event`](CompiledDefinition::apply_event) を通る
//! (`coding-rules/aggregate-commands.md`)。

use std::collections::{BTreeMap, BTreeSet};

use super::compiled_definition_event::{
    Compiled, CompiledDefinitionEvent, PluginSelectionApplied, Recompiled, ScopeRegistered,
};
use super::compiled_definition_id::CompiledDefinitionId;
use super::definition_revision::DefinitionRevision;
use super::plan_action::PlanAction;
use super::plugin_selection_error::PluginSelectionError;
use super::recompile_error::RecompileError;
use super::register_scope_error::RegisterScopeError;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;
use super::stage_slug::StageSlug;

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
    /// と同じ理由)。集約は誕生記録からの変換 (`From<Compiled>`) で導出する — 構築口は
    /// genesis とこの変換だけである (`coding-rules/aggregate-commands.md`「再構成の形」)。
    ///
    /// [`WorkflowDefinitionEvent::Defined`]: super::WorkflowDefinitionEvent
    #[must_use]
    pub fn compile(
        id: CompiledDefinitionId,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> (CompiledDefinition, CompiledDefinitionEvent) {
        let compiled = Compiled::new(id, graph, grid, scopes);
        let compiled_definition = CompiledDefinition::from(compiled.clone());
        (
            compiled_definition,
            CompiledDefinitionEvent::Compiled(compiled),
        )
    }

    /// 源が変わったので内容を入れ替える (再コンパイル — 1 コマンド 1 イベント)。
    ///
    /// 内容が現在と同じなら書くべき事実が無いので `Unchanged` で拒否する — 判断は集約が
    /// 持ち、呼出側に内容の比較を再実装させない。「源が変わっていない」を問うだけの呼出側
    /// (`compile --check`) はこの拒否を答えとして読めばよい。
    ///
    /// # Errors
    ///
    /// 内容が現在と同じ (`Unchanged`)。
    pub fn recompile(
        &mut self,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> Result<CompiledDefinitionEvent, RecompileError> {
        if self.graph == graph && self.grid == grid && self.scopes == scopes {
            return Err(RecompileError::Unchanged {
                revision: self.revision.clone(),
            });
        }
        let event = CompiledDefinitionEvent::Recompiled(Recompiled::new(
            self.id.clone(),
            graph,
            grid,
            scopes,
        ));
        self.apply_event(&event);
        Ok(event)
    }

    /// スコープを 1 つ登記する (identity + グリッド 1 列 — コンポーザ承認時の書込)。
    ///
    /// 有効スコープの権威は identity (12 §4 #6) なので、重複の判定も identity で行う。
    /// 同名の列だけが先に存在する (ランタイム不可視のゴースト列) 場合は登記で列が
    /// 差し替わる — 列は権威ではない。列は空でもよい (zero-EXECUTE スコープ)。
    ///
    /// # Errors
    ///
    /// 同名のスコープが登記済み (`DuplicateScope`)、`freeform_default` を既に別のスコープが
    /// 持つ (`FreeformDefaultAlreadyTaken`)、列がグラフに無いステージを指す (`UnknownStage`)。
    pub fn register_scope(
        &mut self,
        metadata: ScopeMetadata,
        column: BTreeMap<StageSlug, PlanAction>,
    ) -> Result<CompiledDefinitionEvent, RegisterScopeError> {
        if self.scopes.contains_key(metadata.name()) {
            return Err(RegisterScopeError::DuplicateScope {
                name: metadata.name().to_string(),
            });
        }
        if metadata.freeform_default()
            && let Some(holder) = self.scopes.values().find(|scope| scope.freeform_default())
        {
            return Err(RegisterScopeError::FreeformDefaultAlreadyTaken {
                holder: holder.name().to_string(),
            });
        }
        if let Some(slug) = column.keys().find(|slug| self.graph.get(slug).is_none()) {
            return Err(RegisterScopeError::UnknownStage { slug: slug.clone() });
        }
        let event = CompiledDefinitionEvent::ScopeRegistered(ScopeRegistered::new(
            self.id.clone(),
            metadata,
            column,
        ));
        self.apply_event(&event);
        Ok(event)
    }

    /// プラグインの有効・無効の選択を適用する (upstream `select-plugins` の意味論)。
    ///
    /// plugin 所属ノードの `enabled` を一度落とし、選択に**無い**プラグインのノードだけ
    /// `false` を立てる。非プラグインノードは触らない。
    ///
    /// # Errors
    ///
    /// 選択がどのステージも宣言していないプラグインを名指す (`UnknownPlugin`)、適用しても
    /// グラフが変わらない (`Unchanged`)。
    pub fn apply_plugin_selection(
        &mut self,
        enabled_plugins: BTreeSet<String>,
    ) -> Result<CompiledDefinitionEvent, PluginSelectionError> {
        let declared = self.graph.declared_plugins();
        if let Some(name) = enabled_plugins
            .iter()
            .find(|name| !declared.contains(name.as_str()))
        {
            return Err(PluginSelectionError::UnknownPlugin { name: name.clone() });
        }
        if self.graph.with_plugin_selection(&enabled_plugins) == self.graph {
            return Err(PluginSelectionError::Unchanged);
        }
        let event = CompiledDefinitionEvent::PluginSelectionApplied(PluginSelectionApplied::new(
            self.id.clone(),
            enabled_plugins,
        ));
        self.apply_event(&event);
        Ok(event)
    }

    /// イベントを 1 つ適用する (コマンドの唯一の状態遷移経路 — decide / apply の分離)。
    ///
    /// 内容が変わる遷移のあとは内容版を導出し直す。genesis は差分適用では何も変えない —
    /// スナップショット種 (`From<Compiled>`) が誕生を含む。
    fn apply_event(&mut self, event: &CompiledDefinitionEvent) {
        // 変種の網羅 match — 腕の欠落はビルドで落ちる。
        match event {
            CompiledDefinitionEvent::Compiled(_) => {}
            CompiledDefinitionEvent::Recompiled(recompiled) => {
                self.graph = recompiled.graph().clone();
                self.grid = recompiled.grid().clone();
                self.scopes = recompiled.scopes().clone();
            }
            CompiledDefinitionEvent::ScopeRegistered(registered) => {
                let name = registered.metadata().name().to_string();
                self.scopes
                    .insert(name.clone(), registered.metadata().clone());
                self.grid = self
                    .grid
                    .clone()
                    .with_column(name, registered.column().clone());
            }
            CompiledDefinitionEvent::PluginSelectionApplied(applied) => {
                self.graph = self.graph.with_plugin_selection(applied.enabled_plugins());
            }
        }
        self.revision = DefinitionRevision::of_content(&self.graph, &self.grid, &self.scopes);
    }

    /// 配布束が名乗る系譜 ID (`harness.json` の `name` — ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionId {
        &self.id
    }

    /// 内容版 — 内容の純粋な関数で、集約が遷移のたびに導出し直す。
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
}

impl From<Compiled> for CompiledDefinition {
    /// 誕生記録から集約を導出する (リプレイのスナップショット種 — `Intent` の
    /// `From<(Created, occurred_at)>` / `WorkflowDefinition` の `From<(Defined, occurred_at)>`
    /// と対)。
    ///
    /// **構造体リテラルはここだけ** — genesis ([`CompiledDefinition::compile`]) もこの変換を
    /// 通る (`coding-rules/factory-naming.md`「すべての構築経路が基本コンストラクタを通る」)。
    /// Repository の読取経路も、媒体から復号した内容を [`Compiled`] に束ねてここを通す。
    /// 発生時刻を対にしないのは、この集約が通番・版・更新時刻を持たない (媒体がジャーナル
    /// ではない) からである。内容版はここで導出する。
    fn from(compiled: Compiled) -> CompiledDefinition {
        CompiledDefinition {
            id: compiled.id().clone(),
            revision: DefinitionRevision::of_content(
                compiled.graph(),
                compiled.grid(),
                compiled.scopes(),
            ),
            graph: compiled.graph().clone(),
            grid: compiled.grid().clone(),
            scopes: compiled.scopes().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使う (file 単位の allow)。
    #![allow(clippy::panic)]

    use super::*;

    use crate::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNode, StageNodeBuilder, StageNumber,
    };

    fn node(slug: &str, number: &str, plugin: Option<&str>) -> StageNode {
        let mut builder = StageNodeBuilder::new(
            StageSlug::parse(slug).expect("slug"),
            StageNumber::parse(number).expect("番号"),
            slug.to_string(),
            PhaseId::Initialization,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(vec!["classic".to_string()]);
        if let Some(plugin) = plugin {
            builder = builder.plugin(plugin.to_string());
        }
        builder.build()
    }

    fn graph(nodes: Vec<StageNode>) -> StageGraph {
        StageGraph::new(nodes).expect("グラフ")
    }

    fn scopes(names: &[&str]) -> BTreeMap<String, ScopeMetadata> {
        names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    ScopeMetadata::new(name).expect("スコープ"),
                )
            })
            .collect()
    }

    fn id() -> CompiledDefinitionId {
        CompiledDefinitionId::parse("claude").expect("定義 id")
    }

    /// 2 ノード (片方はプラグイン所属) と 1 スコープの配布束。
    fn bundle() -> CompiledDefinition {
        let graph = graph(vec![
            node("state-init", "0.1", None),
            node("acme-audit", "2.9", Some("acme")),
        ]);
        let grid = ScopeGrid::from_graph(&graph);
        CompiledDefinition::compile(id(), graph, grid, scopes(&["classic"])).0
    }

    fn column(slugs: &[(&str, PlanAction)]) -> BTreeMap<StageSlug, PlanAction> {
        slugs
            .iter()
            .map(|(slug, action)| (StageSlug::parse(slug).expect("slug"), *action))
            .collect()
    }

    #[test]
    fn genesis_returns_the_pair_and_the_event_derives_the_same_aggregate() {
        let graph = graph(vec![node("state-init", "0.1", None)]);
        let grid = ScopeGrid::from_graph(&graph);
        let (compiled_definition, event) =
            CompiledDefinition::compile(id(), graph.clone(), grid.clone(), scopes(&["classic"]));
        assert_eq!(compiled_definition.id(), &id());
        assert_eq!(compiled_definition.graph(), &graph);
        assert_eq!(compiled_definition.grid(), &grid);
        assert_eq!(compiled_definition.scopes(), &scopes(&["classic"]));
        // 誕生イベントは材料 (値) を運ぶ — 変換で対の左と同じ集約に戻る。内容版も同じ
        // 内容から同じ値が導出される。
        let CompiledDefinitionEvent::Compiled(compiled) = event else {
            panic!("genesis は Compiled を返す");
        };
        assert_eq!(CompiledDefinition::from(compiled), compiled_definition);
        assert_eq!(
            compiled_definition.revision(),
            &DefinitionRevision::of_content(&graph, &grid, &scopes(&["classic"]))
        );
    }

    #[test]
    fn recompiling_with_new_content_replaces_it_and_moves_the_revision() {
        let mut bundle = bundle();
        let before = bundle.revision().clone();
        let graph = graph(vec![node("state-init", "0.1", None)]);
        let grid = ScopeGrid::from_graph(&graph);

        let event = bundle
            .recompile(graph.clone(), grid.clone(), scopes(&["classic", "feature"]))
            .expect("内容が違えば再コンパイルできる");

        assert!(matches!(event, CompiledDefinitionEvent::Recompiled(_)));
        assert_eq!(bundle.graph(), &graph);
        assert_eq!(bundle.grid(), &grid);
        assert_eq!(bundle.scopes().len(), 2);
        assert_ne!(bundle.revision(), &before, "内容が変われば内容版も変わる");
        assert_eq!(bundle.id(), &id(), "系譜 ID は不変");
    }

    #[test]
    fn recompiling_with_the_same_content_is_refused_without_an_event() {
        let mut bundle = bundle();
        let snapshot = bundle.clone();
        let error = bundle
            .recompile(
                bundle.graph().clone(),
                bundle.grid().clone(),
                bundle.scopes().clone(),
            )
            .expect_err("同じ内容は拒否される");
        assert_eq!(
            error,
            RecompileError::Unchanged {
                revision: snapshot.revision().clone()
            }
        );
        assert_eq!(bundle, snapshot, "拒否された遷移は状態を変えない");
    }

    #[test]
    fn registering_a_scope_adds_its_identity_and_column() {
        let mut bundle = bundle();
        let before = bundle.revision().clone();
        let metadata = ScopeMetadata::new("feature").expect("スコープ");

        let event = bundle
            .register_scope(
                metadata.clone(),
                column(&[
                    ("state-init", PlanAction::Execute),
                    ("acme-audit", PlanAction::Skip),
                ]),
            )
            .expect("新しいスコープは登記できる");

        assert!(matches!(event, CompiledDefinitionEvent::ScopeRegistered(_)));
        assert_eq!(bundle.scopes().get("feature"), Some(&metadata));
        assert_eq!(
            bundle
                .grid()
                .action("feature", &StageSlug::parse("acme-audit").expect("slug")),
            Some(PlanAction::Skip)
        );
        assert_ne!(bundle.revision(), &before);
    }

    #[test]
    fn registering_a_zero_execute_scope_is_allowed() {
        let mut bundle = bundle();
        bundle
            .register_scope(
                ScopeMetadata::new("poc").expect("スコープ"),
                BTreeMap::new(),
            )
            .expect("空列は zero-EXECUTE スコープ (12 §4 #6)");
        assert!(bundle.grid().contains_scope("poc"));
        assert!(bundle.grid().execute_slugs("poc").is_empty());
    }

    #[test]
    fn registering_a_duplicate_scope_is_refused() {
        let mut bundle = bundle();
        let error = bundle
            .register_scope(
                ScopeMetadata::new("classic").expect("スコープ"),
                BTreeMap::new(),
            )
            .expect_err("同名は登記できない");
        assert_eq!(
            error,
            RegisterScopeError::DuplicateScope {
                name: "classic".to_string()
            }
        );
    }

    #[test]
    fn a_second_freeform_default_is_refused_and_names_the_holder() {
        let mut bundle = bundle();
        bundle
            .register_scope(
                ScopeMetadata::new("express")
                    .expect("スコープ")
                    .with_freeform_default(true),
                BTreeMap::new(),
            )
            .expect("最初の freeform_default");
        let error = bundle
            .register_scope(
                ScopeMetadata::new("feature")
                    .expect("スコープ")
                    .with_freeform_default(true),
                BTreeMap::new(),
            )
            .expect_err("freeform_default は 1 つまで");
        assert_eq!(
            error,
            RegisterScopeError::FreeformDefaultAlreadyTaken {
                holder: "express".to_string()
            }
        );
    }

    #[test]
    fn a_column_naming_an_unknown_stage_is_refused() {
        let mut bundle = bundle();
        let error = bundle
            .register_scope(
                ScopeMetadata::new("feature").expect("スコープ"),
                column(&[("nope", PlanAction::Execute)]),
            )
            .expect_err("グラフに無い slug は登記できない");
        assert_eq!(
            error,
            RegisterScopeError::UnknownStage {
                slug: StageSlug::parse("nope").expect("slug")
            }
        );
        assert!(!bundle.grid().contains_scope("feature"));
    }

    #[test]
    fn applying_a_plugin_selection_disables_only_the_unselected_plugin_stages() {
        let mut bundle = bundle();
        let acme = StageSlug::parse("acme-audit").expect("slug");
        assert_eq!(bundle.graph().get(&acme).and_then(StageNode::enabled), None);

        let event = bundle
            .apply_plugin_selection(BTreeSet::new())
            .expect("選択が空なら全プラグインが無効になる");

        assert!(matches!(
            event,
            CompiledDefinitionEvent::PluginSelectionApplied(_)
        ));
        assert_eq!(
            bundle.graph().get(&acme).and_then(StageNode::enabled),
            Some(false)
        );
        let state_init = StageSlug::parse("state-init").expect("slug");
        assert_eq!(
            bundle.graph().get(&state_init).and_then(StageNode::enabled),
            None,
            "非プラグインノードは触らない"
        );

        // 選び直すと `enabled` は落ちる (upstream: 毎回 delete してから無効時のみ false)。
        bundle
            .apply_plugin_selection(["acme".to_string()].into_iter().collect())
            .expect("有効に戻す");
        assert_eq!(bundle.graph().get(&acme).and_then(StageNode::enabled), None);
    }

    #[test]
    fn a_plugin_selection_that_changes_nothing_is_refused() {
        let mut bundle = bundle();
        let error = bundle
            .apply_plugin_selection(["acme".to_string()].into_iter().collect())
            .expect_err("既に有効なので変化なし");
        assert_eq!(error, PluginSelectionError::Unchanged);
    }

    #[test]
    fn an_unknown_plugin_in_the_selection_is_refused() {
        let mut bundle = bundle();
        let error = bundle
            .apply_plugin_selection(["ghost".to_string()].into_iter().collect())
            .expect_err("宣言されていないプラグイン");
        assert_eq!(
            error,
            PluginSelectionError::UnknownPlugin {
                name: "ghost".to_string()
            }
        );
    }

    #[test]
    fn the_revision_is_a_pure_function_of_the_content() {
        // 同じ遷移を別の集約インスタンスに適用すれば同じ内容版に着地する。
        let mut left = bundle();
        let mut right = bundle();
        left.apply_plugin_selection(BTreeSet::new()).expect("遷移");
        right.apply_plugin_selection(BTreeSet::new()).expect("遷移");
        assert_eq!(left.revision(), right.revision());
        assert_eq!(left, right);
    }
}
