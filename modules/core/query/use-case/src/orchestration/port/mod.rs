//! アウトプットポート — クエリ側のインタラクタが依存する**契約 (trait)** と、その契約が
//! 返す DTO (行の写し) の置き場。配置はコマンド側の `port/` と同型である
//! (オーナー裁定 2026-08-31)。
//!
//! # DTO は DAO と同じ port/ に同居する
//!
//! 「Port の Dao が依存する型も port/ にいれて。`*View`」(オーナー裁定 2026-08-31) —
//! **DTO/DAO ポートは一つのパッケージである**。DAO の契約とその契約が返す DTO は同じ理由で
//! 変わる (読取対象のリードモデルが変わったとき) ので、変更の単位を 1 ディレクトリに揃える。
//!
//! # 1 表 1 ポート 1 View
//!
//! 12 のポートはそれぞれ `read_*` 表を**ちょうど 1 つ**引く (オーナー裁定 2026-09-03 —
//! `coding-rules/cqrs-boundaries.md` 規則 6「表の形と読み方」)。JOIN も副問合せも非正規化の
//! 焼き込みも無く、関連は行が運ぶ FK 列で表す。複数の表にまたがる答えは**ユースケースが
//! FK をたどって**組むので、組み立て View は `port/` の住人ではない。
//!
//! # リードモデルは更新できない — 動詞で型保証する
//!
//! どのポートも読取動詞 `find` しか持たない。更新動詞が存在しないことが「リードモデルは
//! 更新できない」の機械強制である (同規則 6)。動詞名 `find` は `gateway-taxonomy.md` §2b の
//! 許容語彙であり、`load` / `get` / `fetch` は使わない。
//!
//! Repository ではなく **DAO** と名乗るのは、読む先が集約ではなくリードモデルだからである
//! (`gateway-taxonomy.md` §3 の 2026-08-31 追記)。
//!
//! # 媒体はポート契約に漏らさない
//!
//! **DAO はファイルや SQLite のテーブルを読んで DTO で返してよい** (オーナー追補裁定
//! 2026-08-31)。どちらを読むかは実装の内部詳細であり、ポート面が語るのは DTO だけである。
//! 媒体名も格納形式もポート名にもシグネチャにも現れない。
//!
//! 実装 (Gateway) は `core-query-interface-adapter` に置く (DIP — `use-case-rules.md` §1)。
//! バインディングはスタティックが既定 (同 §2) なので、ユースケースは型パラメータで DAO を
//! 保持する。
//!
//! 型ファイルの mod も本モジュール自身も private。公開 API は親 (`orchestration`) の
//! `pub use` ファサードが唯一の宣言 (`coding-rules/module-visibility.md`)。

// 契約 (trait) と、そのポート面のエラー
mod definition_dao;
mod execution_dao;
mod jump_dao;
mod jump_phase_dao;
mod next_answer_dao;
mod phase_entry_dao;
mod read_model_read_error;
mod run_stage_dao;
mod scope_change_dao;
mod scope_dao;
mod scope_keyword_dao;
mod steering_part_dao;
mod steering_plan_dao;

// 契約が返す DTO (同居 — オーナー裁定 2026-08-31)
mod read_view;

pub use definition_dao::DefinitionDao;
pub use execution_dao::ExecutionDao;
pub use jump_dao::JumpDao;
pub use jump_phase_dao::JumpPhaseDao;
pub use next_answer_dao::NextAnswerDao;
pub use phase_entry_dao::PhaseEntryDao;
pub use run_stage_dao::RunStageDao;
pub use scope_change_dao::ScopeChangeDao;
pub use scope_dao::ScopeDao;
pub use scope_keyword_dao::ScopeKeywordDao;
pub use steering_part_dao::SteeringPartDao;
pub use steering_plan_dao::SteeringPlanDao;

pub use read_model_read_error::ReadModelReadError;

pub use read_view::{
    DefinitionSummaryView, ExecutionView, JumpPhaseView, JumpView, NextAnswerView, PhaseEntryView,
    RunStageView, ScopeChangeView, ScopeView, SteeringPartView, SteeringPlanView,
};
