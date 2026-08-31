//! アウトプットポート — クエリ側のインタラクタが依存する**契約 (trait)** と、その契約に
//! 依存する型 (ポート面のエラー) の置き場。配置はコマンド側の `port/` と同型である
//! (オーナー裁定 2026-08-31)。
//!
//! # なぜクエリ側にポートがあるのか
//!
//! 「クエリサイドのインターフェイスアダプター層にはリードモデルを読む DTO や DAO が存在する。
//! それらをユースケースから**ポートを経由して**読む形になる。リードモデルは更新できない」
//! (オーナー裁定 2026-08-31)。b26 の「ポートを 1 本も持たず読取結果を `execute` の引数で
//! 値渡しする」形は、`use-case-rules.md` §4 の 2026-08-30 夕・再々裁定 (コマンド側の
//! 読取専用ポート注入の失効) をクエリ側へ誤って適用したものであり、本裁定が置き換える。
//!
//! # リードモデルは更新できない — 動詞で型保証する
//!
//! 3 つのポートはいずれも読取動詞 `find` **1 本だけ**を持つ。更新動詞が存在しないことが
//! 「リードモデルは更新できない」の機械強制である (`coding-rules/cqrs-boundaries.md`
//! 規則 6)。動詞名 `find` は `gateway-taxonomy.md` §2b の許容語彙であり、`load` / `get` /
//! `fetch` は使わない。
//!
//! Repository ではなく **DAO** と名乗るのは、読む先が集約ではなくリードモデルだからである
//! (`gateway-taxonomy.md` §3 の 2026-08-31 追記 — クエリ側のリードモデル読取ポートは
//! `XxxDao`、実装は `XxxDaoImpl`、ダブルは `InMemoryXxxDao`)。
//!
//! # 媒体はポート契約に漏らさない
//!
//! **DAO はファイルや SQLite のテーブルを読んで DTO で返してよい** (オーナー追補裁定
//! 2026-08-31)。どちらを読むかは実装の内部詳細であり、ポート面が語るのは DTO
//! (クエリモデル — [`ExecutionStateView`] / [`DefinitionView`] / [`MemoryRules`]) だけである。
//! 媒体名も格納形式もポート名にもシグネチャにも現れない (`gateway-taxonomy.md` §2 が
//! Repository に課す媒体名禁止と同じ理屈 — 格納形式が変わってもポート面が動かないこと自体が
//! 目的である)。現行の実装 3 本はいずれもファイルを読むが、それは**いま選んでいる媒体**で
//! あって契約ではない。
//!
//! [`ExecutionStateView`]: crate::execution_view::ExecutionStateView
//! [`DefinitionView`]: crate::workflow_view::DefinitionView
//! [`MemoryRules`]: crate::orchestration::MemoryRules
//!
//! 実装 (Gateway) は `core-query-interface-adapter` に置く (DIP — `use-case-rules.md` §1)。
//! バインディングはスタティックが既定 (同 §2) なので、ユースケースは型パラメータで DAO を
//! 保持する。
//!
//! 型ファイルの mod は private。公開 API は親モジュールの `pub use` ファサードが唯一の宣言
//! (`coding-rules/module-visibility.md`)。

mod execution_state_dao;
mod execution_state_read_error;
mod memory_rules_dao;
mod memory_rules_read_error;
mod workflow_definition_dao;
mod workflow_definition_read_error;

pub use execution_state_dao::ExecutionStateDao;
pub use execution_state_read_error::ExecutionStateReadError;
pub use memory_rules_dao::MemoryRulesDao;
pub use memory_rules_read_error::MemoryRulesReadError;
pub use workflow_definition_dao::WorkflowDefinitionDao;
pub use workflow_definition_read_error::WorkflowDefinitionReadError;
