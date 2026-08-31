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
//! 読む対象は 2 つある。**ワークフロー定義**の 3 入力 (compile コンテキストの投影) と、
//! **実行状態** `aidlc-state.md` (orchestration の投影) である。加えて、`next` / `continue`
//! が交換する **continue_token** の封緘・開封もこの層が持つ — HMAC 封筒と 18 キーの
//! ワイヤ形式は upstream の輸送形であって、クエリモデルの語彙ではない
//! (`coding-rules/upstream-contracts.md`「境界で変換」)。
//!
//! 層の分業: reader ([`load_workflow_definition`] / [`load_execution_state`]) がパス解決・
//! ファイル読取・ディレクトリ列挙を行い、parse ([`parse_workflow_definition`] /
//! [`parse_execution_state`]) は **fs 呼び出しゼロ**の変換だけを持つ。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。

mod continue_token_wire;
mod execution_state_parse;
mod execution_state_reader;
mod workflow_definition_parse;
mod workflow_definition_reader;

pub use continue_token_wire::{InvalidContinueToken, mint_continue_token, verify_continue_token};
pub use execution_state_parse::{ExecutionStateParseError, parse_execution_state};
pub use execution_state_reader::{
    ExecutionStateReadError, LoadedExecutionState, load_execution_state, state_file_path,
};
pub use workflow_definition_parse::{
    DefinitionArtifacts, GraphReadError, RawArtifact, graph_read_error_message,
    parse_workflow_definition, stage_graph_invalid_json_message, stage_graph_not_readable_message,
};
pub use workflow_definition_reader::{DefinitionPaths, load_workflow_definition};
