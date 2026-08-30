//! クエリサイドのインターフェイスアダプタ — **リードモデルを読む側**。
//!
//! `stage-graph.json` は compile コンテキストのイベント投影 = リードモデルであり、これを
//! 読む・パースする責務は**クエリサイドの実装**である (オーナー裁定 2026-08-30 — コマンド
//! 側に置くのは CQRS 違反)。コマンド側 3 クレートはこのクレートを知らない。読んだ結果
//! (`WorkflowDefinition` の値) をコマンドのユースケースへ渡すのは、両側を知る合成ルート
//! (`modules/app/aidlc`) だけである。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。

mod workflow_definition_parse;
mod workflow_definition_reader;

pub use workflow_definition_parse::{
    DefinitionArtifacts, GraphReadError, RawArtifact, graph_read_error_message,
    parse_workflow_definition, stage_graph_invalid_json_message, stage_graph_not_readable_message,
};
pub use workflow_definition_reader::{DefinitionPaths, load_workflow_definition};
