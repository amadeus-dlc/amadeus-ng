//! アウトプットポート — クエリ側のインタラクタが依存する**契約 (trait)** と、その契約に
//! 依存する型 (ポート面のエラーと、DAO が返す DTO) の置き場。配置はコマンド側の `port/` と
//! 同型である (オーナー裁定 2026-08-31)。
//!
//! # DTO は DAO と同じ port/ に同居する
//!
//! 「Port の Dao が依存する型も port/ にいれて。`*View`」(オーナー裁定 2026-08-31) —
//! **DTO/DAO ポートは一つのパッケージである**。DAO の契約とその契約が返す DTO は同じ理由で
//! 変わる (読取対象のリードモデルが変わったとき) ので、変更の単位を 1 ディレクトリに揃える。
//! DTO は読む対象ごとに 2 族に分かれる:
//!
//! - `workflow_view` — ワークフロー定義リードモデル (3 入力) のビュー型
//! - `execution_view` — 実行状態リードモデル (`aidlc-state.md`) のビュー型と、その上の判断
//!   (BR3.1 の 8 分岐)
//!
//! `MemoryRules` は `MemoryRulesDao` の戻り値 DTO なので同じ理由でここに住むが、`View`
//! 接尾辞を持たない — 読むのは memory 層のルール本文であってリードモデルの写しではない。
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
//! [`ExecutionStateView`]: crate::orchestration::ExecutionStateView
//! [`DefinitionView`]: crate::orchestration::DefinitionView
//! [`MemoryRules`]: crate::orchestration::MemoryRules
//!
//! 実装 (Gateway) は `core-query-interface-adapter` に置く (DIP — `use-case-rules.md` §1)。
//! バインディングはスタティックが既定 (同 §2) なので、ユースケースは型パラメータで DAO を
//! 保持する。
//!
//! 型ファイルの mod は private。公開 API は親モジュールの `pub use` ファサードが唯一の宣言
//! (`coding-rules/module-visibility.md`)。

// 契約 (trait) と、そのポート面のエラー
mod definition_dao;
mod execution_dao;
mod execution_state_dao;
mod execution_state_read_error;
mod jump_dao;
mod jump_phase_dao;
mod memory_rules_dao;
mod memory_rules_read_error;
mod next_answer_dao;
mod phase_entry_dao;
mod read_model_read_error;
mod run_stage_dao;
mod scope_change_dao;
mod scope_dao;
mod scope_keyword_dao;
mod steering_part_dao;
mod steering_plan_dao;
mod workflow_definition_dao;
mod workflow_definition_read_error;

// 契約が返す DTO (同居 — オーナー裁定 2026-08-31)
mod execution_view;
mod memory_rules;
mod read_view;
mod workflow_view;

pub use execution_state_dao::ExecutionStateDao;
pub use execution_state_read_error::ExecutionStateReadError;
pub use memory_rules_dao::MemoryRulesDao;
pub use memory_rules_read_error::MemoryRulesReadError;
pub use workflow_definition_dao::WorkflowDefinitionDao;
pub use workflow_definition_read_error::WorkflowDefinitionReadError;

// --- 構造化リードモデル (`read_*` 表) を引く 12 ポートと、その共通の読取失敗 ---
//
// **1 表 1 ポート**である (オーナー裁定 2026-09-03 — DAO は 1 表 1 引当)。複数の表に
// またがる答えは、ユースケースが FK をたどって表ごとに引いて組む。

pub use definition_dao::DefinitionDao;
pub use execution_dao::ExecutionDao;
pub use jump_dao::JumpDao;
pub use jump_phase_dao::JumpPhaseDao;
pub use next_answer_dao::NextAnswerDao;
pub use phase_entry_dao::PhaseEntryDao;
pub use read_model_read_error::ReadModelReadError;
pub use run_stage_dao::RunStageDao;
pub use scope_change_dao::ScopeChangeDao;
pub use scope_dao::ScopeDao;
pub use scope_keyword_dao::ScopeKeywordDao;
pub use steering_part_dao::SteeringPartDao;
pub use steering_plan_dao::SteeringPlanDao;

// --- DTO: 構造化リードモデルの行の写し (`read_view`) ---

pub use read_view::{
    DefinitionSummaryView, ExecutionView, JumpPhaseView, JumpView, NextAnswerView, PhaseEntryView,
    RunStageView, ScopeChangeView, ScopeView, SteeringPartView, SteeringPlanView,
};

// --- DTO: ワークフロー定義リードモデルのビュー型 (`workflow_view`) ---

// 閉集合の語彙
pub use workflow_view::{
    BrownfieldGreenfieldView, ExecutionKindView, PhaseView, PlanActionView, ReviewCapValueView,
    ReviewClassView, RuleScopeView, SkeletonDefaultView, StageModeView,
};
// 検証付きの値
pub use workflow_view::{
    DefinitionIdView, DefinitionRevisionView, ScopeSlugView, StageNumberView, StageSlugView,
};
// レコード
pub use workflow_view::{
    ConsumeDeclView, RuleInContextView, ScopeMetadataView, SensorRefView, StageRouteView,
    StageView, StageViewBuilder,
};
// リードモデル本体
pub use workflow_view::{DefinitionView, ScopeGridView, StageGraphView};
// 拒否 (ビューではないので `View` 接尾辞を付けない)
pub use workflow_view::{
    DefinitionIdError, DefinitionRevisionError, ScopeMetadataError, ScopeSlugError,
    StageGraphError, StageNumberError, StageSlugError, UnknownScope, UnknownValue,
};

// --- DTO: 実行状態リードモデルのビュー型 (`execution_view`) ---

// 閉集合の語彙と位置
pub use execution_view::{CheckboxState, ExecutionStatus, StageIndex};
// リードモデル本体
pub use execution_view::{ExecutionStateView, StageProgressView};
// 拒否
pub use execution_view::ExecutionStateError;

// --- DTO: memory 層ルール束 (`MemoryRulesDao` の戻り値) ---
pub use memory_rules::MemoryRules;
