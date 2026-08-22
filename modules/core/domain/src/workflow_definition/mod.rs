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

pub mod execution_kind;
pub mod phase;
pub mod review_class;
pub mod scope_grid;
pub mod scope_metadata;
pub mod stage_graph;
pub mod stage_mode;
pub mod stage_node;
pub mod stage_number;
pub mod stage_slug;
// 集約名とモジュール名が一致する意図的な構成 (集約の正本ファイル)。
#[allow(clippy::module_inception)]
pub mod workflow_definition;
