//! **クエリ側**のインターフェイスアダプタ層 — リードモデルを読む I/O 面。
//!
//! `stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md` は compile コンテキストの
//! イベントから RMU が作成・更新する**リードモデル**であり、これを読む・パースする責務は
//! クエリ側にある (オーナー裁定 2026-08-30 — コマンド側に置くのは CQRS 違反。
//! `coding-rules/cqrs-boundaries.md` 規則 7)。
//!
//! 読んだ結果は**自前のクエリモデル**([`core_query_use_case::orchestration`] の `~View` 型) に
//! 写す。クエリ側のユースケース (`next` / `continue` のような読むだけの動詞) はこのビューを
//! 消費する。コマンド側は同じ Published Language を読むにしても、自分の
//! `WorkflowDefinitionRepository` で自分の集約を再構成し、書込ユースケースの中でだけ使う —
//! 両者は**側ごと専用の別実装**であって、一方が他方の読取結果を受け取ることはない
//! (同規則「共有部品は側の独立を DRY に優先」)。
//!
//! # DAO 実装 3 本
//!
//! ユースケースはリードモデルを**ポート経由で**読む (オーナー裁定 2026-08-31)。本クレートは
//! その 3 ポートの実 Gateway を持つ。実装名は `XxxDaoImpl`、テストダブルは `InMemoryXxxDao`
//! (`coding-rules/gateway-taxonomy.md` §3 の 2026-08-31 追記 — `Impl` 接尾辞は本物の印)。
//!
//! | ポート | 実 Gateway | 読む先 |
//! | --- | --- | --- |
//! | `WorkflowDefinitionDao` | [`WorkflowDefinitionDaoImpl`] | 定義 3 入力 + scope identity 群 |
//! | `ExecutionStateDao` | [`ExecutionStateDaoImpl`] | `aidlc-state.md` (orchestration の投影) |
//! | `MemoryRulesDao` | [`MemoryRulesDaoImpl`] | active-space の memory 層 |
//!
//! **「読む先」の欄はこの層の持ち物である。** DAO はファイルでも SQLite のテーブルでも読んで
//! DTO で返してよく、どちらを読むかは実装の内部詳細であってポート契約には現れない
//! (オーナー追補裁定 2026-08-31 — `coding-rules/gateway-taxonomy.md` §3 の DAO 項)。
//! 現行 3 本はいずれもファイルを読むが、それは**いま選んでいる媒体**であって契約ではない —
//! 格納形式を替えても差し替わるのはこのクレートだけである。
//!
//! 加えて、`next` / `continue` が交換する **continue_token** の封緘・開封もこの層が持つ —
//! HMAC 封筒と 18 キーのワイヤ形式は upstream の輸送形であって、クエリモデルの語彙ではない
//! (`coding-rules/upstream-contracts.md`「境界で変換」)。トークンは**リードモデルではなく
//! 要求素材**なので、DAO ポートではなくユースケースの引数として渡る。
//!
//! 層の分業: DAO 実装がパス解決・ファイル読取・ディレクトリ列挙を行い、parse
//! ([`parse_workflow_definition`] / [`parse_execution_state`]) は **fs 呼び出しゼロ**の変換
//! だけを持つ。**I/O 失敗の逐語文言はここには無い** — ポートが運ぶのは材料だけで、文言を
//! 組むのは出す側のユースケースである (`coding-rules/error-handling.md`)。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。

mod continue_token_dto;
mod execution_state_dao_impl;
mod execution_state_parse;
mod memory;
mod memory_rules_dao_impl;
mod workflow_definition_dao_impl;
mod workflow_definition_parse;

// 実 Gateway (DAO 実装) — ポート (trait) は `core_query_use_case::orchestration` が所有する。
pub use execution_state_dao_impl::ExecutionStateDaoImpl;
pub use memory_rules_dao_impl::MemoryRulesDaoImpl;
pub use workflow_definition_dao_impl::{DefinitionPaths, WorkflowDefinitionDaoImpl};

// テスト用 in-memory 実装 (合成ルートとその周辺のテストが実 I/O 無しで組むための口)。
pub use memory::{
    InMemoryExecutionStateDao, InMemoryMemoryRulesDao, InMemoryWorkflowDefinitionDao,
};

// 継続トークンの封緘・開封 (輸送形の境界)。
pub use continue_token_dto::{InvalidContinueToken, mint_continue_token, verify_continue_token};

// 純 parse (fs 呼び出しゼロ) — DAO 実装の下請けだが、ゴールデンパリティテストが
// 読み終えたバイトを直接与えるために公開する。
pub use execution_state_parse::{ExecutionStateParseError, parse_execution_state};
pub use workflow_definition_parse::{DefinitionArtifacts, RawArtifact, parse_workflow_definition};
