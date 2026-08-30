//! ワークフロー定義リードモデルのビュー型 — クエリ側が読むための自前モデル。
//!
//! 読む対象は 3 入力 (`stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md`)
//! で、I/O と復号はクエリ側のインターフェイスアダプタ (`core-query-interface-adapter`) が
//! 行う。本モジュールの型は**ワイヤ形式を知らない** — 直列化の記述を持たず、検証済みの値
//! だけを保持する。
//!
//! **命名**: リードモデルのデータを表す型は `~View` 接尾辞を付け、クエリ側の語彙であることを
//! 明示する。拒否 (エラー) 型はビューではないので接尾辞を付けない。
//!
//! **閉集合は厳密**である (12 §10 表 #3 — 2026-08-22 裁定)。`phase` / `execution` /
//! `review_class` / `mode` などの未知値は読取時に落とし、`Unknown` 変種へ逃がさない。
//! 観測差は手編集グラフに限られ、配布実バイトでは生じないことをゴールデンパリティが固定する。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、消費側の
//! パスは `core_query_use_case::workflow_view::<型>` で安定する
//! (`coding-rules/module-visibility.md`)。

mod brownfield_greenfield_view;
mod consume_decl_view;
mod definition_id_view;
mod definition_revision_view;
mod definition_view;
mod execution_kind_view;
mod phase_view;
mod plan_action_view;
mod review_cap_value_view;
mod review_class_view;
mod rule_in_context_view;
mod rule_scope_view;
mod scope_grid_view;
mod scope_metadata_view;
mod sensor_ref_view;
mod skeleton_default_view;
mod stage_graph_view;
mod stage_mode_view;
mod stage_number_view;
mod stage_slug_view;
mod stage_view;
mod unknown_value;

// 閉集合の語彙
pub use brownfield_greenfield_view::BrownfieldGreenfieldView;
pub use execution_kind_view::ExecutionKindView;
pub use phase_view::PhaseView;
pub use plan_action_view::PlanActionView;
pub use review_cap_value_view::ReviewCapValueView;
pub use review_class_view::ReviewClassView;
pub use rule_scope_view::RuleScopeView;
pub use skeleton_default_view::SkeletonDefaultView;
pub use stage_mode_view::StageModeView;

// 検証付きの値
pub use definition_id_view::DefinitionIdView;
pub use definition_revision_view::DefinitionRevisionView;
pub use stage_number_view::StageNumberView;
pub use stage_slug_view::StageSlugView;

// レコード
pub use consume_decl_view::ConsumeDeclView;
pub use rule_in_context_view::RuleInContextView;
pub use scope_metadata_view::ScopeMetadataView;
pub use sensor_ref_view::SensorRefView;
pub use stage_view::{StageView, StageViewBuilder};

// リードモデル本体
pub use definition_view::DefinitionView;
pub use scope_grid_view::ScopeGridView;
pub use stage_graph_view::StageGraphView;

// 拒否 (ビューではないので `View` 接尾辞を付けない)
pub use definition_id_view::DefinitionIdError;
pub use definition_revision_view::DefinitionRevisionError;
pub use definition_view::UnknownScope;
pub use scope_metadata_view::ScopeMetadataError;
pub use stage_graph_view::StageGraphError;
pub use stage_number_view::StageNumberError;
pub use stage_slug_view::StageSlugError;
pub use unknown_value::UnknownValue;
