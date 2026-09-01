//! workflow-definition コンテキスト (01 §3.1) — 「何を実行しうるか」の静的定義。
//!
//! 本モジュールは Published Language 1 本目 (コンパイル済み `stage-graph.json` /
//! `scope-grid.json`、および有効スコープの権威である `scopes/aidlc-<name>.md`) の
//! **読取モデル**を担う。ワイヤ形式のデコード (JSON / frontmatter パース) は Gateway 層の
//! 責務であり、本モジュールの型はその形を知らない — 直列化の記述も持たない (改訂 9 /
//! `coding-rules/domain-persistence-neutrality.md`)。`StageSlug` / `WorkflowDefinitionId` /
//! `DefinitionRevision` を復号するアダプタ層の DTO は `parse` を通すので、Always Valid は
//! そこでも破れない。
//!
//! 契約の逐語根拠は Issue #7 項目 3 の抽出レポート。要点:
//! - `stage-graph.json` のルートは**配列**で、ノードは `FIELD_ORDER` 28 フィールド。
//! - `scope-grid.json` は `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` の純粋な転置。
//! - 有効スコープの権威は `.md` の存在であり、グリッドではない。
//! - 未知スコープの扱いは述語ごとに**非対称** (`WorkflowDefinition` の doc を参照)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_domain::workflow_definition::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod brownfield_greenfield;
mod consume_decl;
mod definition_revision;
mod definition_revision_error;
mod execution_kind;
mod phase_id;
mod plan_action;
mod redefine_error;
mod review_cap_value;
mod review_class;
mod rule_in_context;
mod rule_scope;
mod scope_grid;
mod scope_metadata;
mod scope_metadata_error;
mod scope_slug;
mod scope_slug_error;
mod sensor_ref;
mod skeleton_default;
mod stage_graph;
mod stage_graph_error;
mod stage_mode;
mod stage_node;
mod stage_number;
mod stage_number_error;
mod stage_route;
mod stage_slug;
mod stage_slug_error;
mod unknown_brownfield_greenfield;
mod unknown_execution_kind;
mod unknown_phase;
mod unknown_review_cap;
mod unknown_review_class;
mod unknown_rule_scope;
mod unknown_scope;
mod unknown_skeleton_default;
mod unknown_stage_mode;
mod workflow_definition_event;
mod workflow_definition_id;
mod workflow_definition_id_error;
// 集約名とモジュール名が一致する意図的な構成 (集約の正本ファイル)。
#[allow(clippy::module_inception)]
mod workflow_definition;

// ドメインイベント (定義集約の genesis と改訂 — coding-rules/aggregate-commands.md)
pub use workflow_definition_event::{Defined, Redefined, WorkflowDefinitionEvent};

// Domain Primitive
pub use brownfield_greenfield::BrownfieldGreenfield;
pub use consume_decl::ConsumeDecl;
pub use definition_revision::DefinitionRevision;
pub use execution_kind::ExecutionKind;
pub use phase_id::PhaseId;
pub use plan_action::PlanAction;
pub use review_cap_value::ReviewCapValue;
pub use review_class::ReviewClass;
pub use rule_in_context::RuleInContext;
pub use rule_scope::RuleScope;
pub use scope_metadata::ScopeMetadata;
pub use scope_slug::ScopeSlug;
pub use sensor_ref::SensorRef;
pub use skeleton_default::SkeletonDefault;
pub use stage_mode::StageMode;
pub use stage_number::StageNumber;
pub use stage_route::StageRoute;
pub use stage_slug::StageSlug;
pub use workflow_definition_id::WorkflowDefinitionId;

// 読取モデル (Published Language 1 本目)
pub use scope_grid::ScopeGrid;
pub use stage_graph::StageGraph;
pub use stage_node::StageNode;
pub use workflow_definition::WorkflowDefinition;

// ビルダー
pub use stage_node::StageNodeBuilder;

// エラー
pub use definition_revision_error::DefinitionRevisionError;
pub use redefine_error::RedefineError;
pub use scope_metadata_error::ScopeMetadataError;
pub use scope_slug_error::ScopeSlugError;
pub use stage_graph_error::StageGraphError;
pub use stage_number_error::StageNumberError;
pub use stage_slug_error::StageSlugError;
pub use unknown_brownfield_greenfield::UnknownBrownfieldGreenfield;
pub use unknown_execution_kind::UnknownExecutionKind;
pub use unknown_phase::UnknownPhase;
pub use unknown_review_cap::UnknownReviewCap;
pub use unknown_review_class::UnknownReviewClass;
pub use unknown_rule_scope::UnknownRuleScope;
pub use unknown_scope::UnknownScope;
pub use unknown_skeleton_default::UnknownSkeletonDefault;
pub use unknown_stage_mode::UnknownStageMode;
pub use workflow_definition_id_error::WorkflowDefinitionIdError;
