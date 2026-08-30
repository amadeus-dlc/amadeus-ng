//! **クエリ側**のインターフェイスアダプタ層 — リードモデルを読む I/O 面。
//!
//! `stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md` は compile コンテキストの
//! イベントから RMU が作成・更新する**リードモデル**であり、これを読む・パースする責務は
//! クエリ側にある (オーナー裁定 2026-08-30 — コマンド側に置くのは CQRS 違反。
//! `coding-rules/cqrs-boundaries.md` 規則 7)。
//!
//! 読んだ結果は**自前のクエリモデル**([`core_query_use_case::workflow_view`] のビュー型) に
//! 写す。クエリ側のユースケース (`next` / `continue` のような読むだけの動詞) はこのビューを
//! 消費する。コマンド側は同じ Published Language を読むにしても、自分の
//! `WorkflowDefinitionRepository` で自分の集約を再構成し、書込ユースケースの中でだけ使う —
//! 両者は**側ごと専用の別実装**であって、一方が他方の読取結果を受け取ることはない
//! (同規則「共有部品は側の独立を DRY に優先」)。
//!
//! 層の分業: [`load_workflow_definition`] がパス解決・ファイル読取・ディレクトリ列挙を行い、
//! [`parse_workflow_definition`] は **fs 呼び出しゼロ**の変換だけを持つ。
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
