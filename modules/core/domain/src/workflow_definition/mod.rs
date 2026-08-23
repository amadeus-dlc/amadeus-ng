//! workflow-definition コンテキスト (01 §3.1) — 「何を実行しうるか」の静的定義。
//!
//! 本モジュールは Published Language 1 本目 (コンパイル済み `stage-graph.json` /
//! `scope-grid.json`、および有効スコープの権威である `scopes/aidlc-<name>.md`) の
//! **読取モデル**を担う。ワイヤ形式のデコード (JSON / frontmatter パース) は Gateway 層の
//! 責務で、ドメイン層は serde に依存しない。
//!
//! 契約の逐語根拠は Issue #7 項目 3 の抽出レポート。要点:
//! - `stage-graph.json` のルートは**配列**で、ノードは `FIELD_ORDER` 28 フィールド。
//! - `scope-grid.json` は `{ <scope>: { stages: { <slug>: "EXECUTE"|"SKIP" } } }` の純粋な転置。
//! - 有効スコープの権威は `.md` の存在であり、グリッドではない。
//! - 未知スコープの扱いは述語ごとに**非対称** (`WorkflowDefinition` の doc を参照)。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_domain::workflow_definition::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod definition_revision;
mod execution_kind;
mod phase;
mod plan_action;
mod review_class;
mod scope_grid;
mod scope_metadata;
mod stage_graph;
mod stage_mode;
mod stage_node;
mod stage_number;
mod stage_slug;
mod workflow_definition_id;
// 集約名とモジュール名が一致する意図的な構成 (集約の正本ファイル)。
#[allow(clippy::module_inception)]
mod workflow_definition;

// Domain Primitive
pub use definition_revision::DefinitionRevision;
pub use execution_kind::ExecutionKind;
pub use phase::PhaseId;
pub use plan_action::PlanAction;
pub use review_class::ReviewClass;
pub use scope_metadata::{ReviewCapValue, ScopeMetadata, SkeletonDefault};
pub use stage_mode::StageMode;
pub use stage_node::{BrownfieldGreenfield, ConsumeDecl, RuleInContext, RuleScope, SensorRef};
pub use stage_number::StageNumber;
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
pub use definition_revision::DefinitionRevisionError;
pub use execution_kind::UnknownExecutionKind;
pub use phase::UnknownPhase;
pub use review_class::UnknownReviewClass;
pub use scope_metadata::{ScopeMetadataError, UnknownReviewCap, UnknownSkeletonDefault};
pub use stage_graph::StageGraphError;
pub use stage_mode::UnknownStageMode;
pub use stage_node::{UnknownBrownfieldGreenfield, UnknownRuleScope};
pub use stage_number::StageNumberError;
pub use stage_slug::StageSlugError;
pub use workflow_definition::UnknownScope;
pub use workflow_definition_id::WorkflowDefinitionIdError;
