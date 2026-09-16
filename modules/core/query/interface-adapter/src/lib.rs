//! **クエリ側**のインターフェイスアダプタ層 — 構造化リードモデルを読む I/O 面。
//!
//! `read_*` 表は RMU が集約のクエリの答えを非正規化して書いたリードモデルであり、これを
//! 読む責務はクエリ側にある (`coding-rules/cqrs-boundaries.md` 規則 6 / 7)。読んだ結果は
//! **自前のクエリモデル** ([`core_query_use_case::orchestration`] の `*View` 型) に写す。
//! コマンド側は同じ Published Language を読むにしても自分の Repository で自分の集約を
//! 再構成する — 両者は**側ごと専用の別実装**である (同規則「共有部品は側の独立を DRY に
//! 優先」)。
//!
//! # DAO 実装 12 本 = 12 表
//!
//! ユースケースはリードモデルを**ポート経由で**読む (オーナー裁定 2026-08-31)。本クレートは
//! その 12 ポートの実 Gateway を持つ。実装名は `XxxDaoImpl`、テストダブルは `InMemoryXxxDao`
//! (`coding-rules/gateway-taxonomy.md` §3 の 2026-08-31 追記 — `Impl` 接尾辞は本物の印)。
//!
//! **1 実装 = 1 表**である (オーナー裁定 2026-09-03)。どの SQL も `read_*` 表を 1 つしか
//! 読まない — JOIN も副問合せも無く、関連は行が運ぶ FK 列で表す。複数の表にまたがる答えは
//! ユースケースが FK をたどって組む。機械強制は `cargo lint` の `dao-single-table`。
//!
//! **媒体 (SQLite) はこの層の内部詳細**であり、ポート面には現れない (同 §3 の DAO 項) —
//! 格納形式を替えても差し替わるのはこのクレートだけである。
//!
//! 12 実装の**唯一の構築経路**は [`ReadModelDaos`] である — 1 要求ぶんの読取専用接続を
//! 1 度だけ開き、12 実装がそれを分け合う。実装ごとに開くと、多段の引当が別々のスナップ
//! ショットを見る余地が残る。
//!
//! 加えて、`next` / `continue` が交換する **continue_token** の封緘・開封もこの層が持つ —
//! HMAC 封筒と 18 キーのワイヤ形式は upstream の輸送形であって、クエリモデルの語彙ではない
//! (`coding-rules/upstream-contracts.md`「境界で変換」)。トークンは**リードモデルではなく
//! 要求素材**なので、DAO ポートではなくユースケースの引数として渡る。
//!
//! **I/O 失敗の逐語文言はここには無い** — ポートが運ぶのは材料だけで、文言を組むのは出す側
//! (合成ルートのプレゼンタ) である (`coding-rules/error-handling.md`)。
//!
//! 実装ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。

mod codekb_scope_dao_impl;
mod codekb_source_fingerprint_dao_impl;
mod continue_token_dto;
mod definition_dao_impl;
mod definition_stage_dao_impl;
mod display_slug;
mod doctor_check_dao_impl;
mod doctor_environment;
mod doctor_observation_dao_impl;
mod doctor_paths;
mod doctor_report_dao_impl;
mod document_input_dao_impl;
mod execution_dao_impl;
mod hook_health_dao_impl;
mod intent_listing_dao_impl;
mod intent_repos_dao_impl;
mod jump_dao_impl;
mod jump_phase_dao_impl;
mod memory;
mod native_doctor_facts;
mod next_answer_dao_impl;
mod phase_entry_dao_impl;
mod project_description_dao_impl;
mod read_model_daos;
mod read_model_failure;
mod read_model_store;
mod registry_row;
mod run_stage_columns;
mod run_stage_dao_impl;
mod scope_change_dao_impl;
mod scope_dao_impl;
mod scope_grid_dao_impl;
mod scope_keyword_dao_impl;
mod scope_metadata_dao_impl;
mod stage_graph_dao_impl;
mod state_file_dao_impl;
mod state_version_classifier;
mod steering_part_dao_impl;
mod steering_plan_dao_impl;

pub use hook_health_dao_impl::HookHealthDaoImpl;

// 12 実装を建てる唯一の口 (1 要求 = 1 接続)。
pub use read_model_daos::ReadModelDaos;

// 実 Gateway (DAO 実装) — ポート (trait) は `core_query_use_case::orchestration` が所有する。
pub use definition_dao_impl::DefinitionDaoImpl;
pub use definition_stage_dao_impl::DefinitionStageDaoImpl;
pub use execution_dao_impl::ExecutionDaoImpl;
pub use jump_dao_impl::JumpDaoImpl;
pub use jump_phase_dao_impl::JumpPhaseDaoImpl;
pub use next_answer_dao_impl::NextAnswerDaoImpl;
pub use phase_entry_dao_impl::PhaseEntryDaoImpl;
pub use run_stage_dao_impl::RunStageDaoImpl;
pub use scope_change_dao_impl::ScopeChangeDaoImpl;
pub use scope_dao_impl::ScopeDaoImpl;
pub use scope_keyword_dao_impl::ScopeKeywordDaoImpl;
pub use steering_part_dao_impl::SteeringPartDaoImpl;
pub use steering_plan_dao_impl::SteeringPlanDaoImpl;

// upstream 互換の人間可読リードモデル (`aidlc-state.md`) を読む実 Gateway。SQLite の
// `read_*` 表とは別の面なので `ReadModelDaos` の住人ではない (b46)。
pub use state_file_dao_impl::StateFileDaoImpl;

// 依頼原文サイドカー (`project-description.json`) を読む実 Gateway。状態ファイルと同じ
// 「upstream 互換・人間可読」の面なので、`ReadModelDaos` の住人ではない (群 B)。
pub use project_description_dao_impl::ProjectDescriptionDaoImpl;

// 直接入力の 2 面 (転送ファイルと、そこが名指す 1 ファイル) を読む実 Gateway。顧客が書いた
// バイトをプロジェクトルートへ封じ込めるのがこの実装の要であり、同じく `ReadModelDaos` の
// 住人ではない (群 B)。
pub use document_input_dao_impl::DocumentInputDaoImpl;

// codekb の読取面 (群 C) — 走査範囲ブロック・intent 登録簿・作業ツリーの内容指紋。いずれも
// SQLite の `read_*` 表とは別の面なので `ReadModelDaos` の住人ではない。
pub use codekb_scope_dao_impl::CodekbScopeDaoImpl;
pub use codekb_source_fingerprint_dao_impl::CodekbSourceFingerprintDaoImpl;
pub use intent_listing_dao_impl::IntentListingDaoImpl;
pub use intent_repos_dao_impl::IntentReposDaoImpl;

// 登録簿に行が無い記録の表示名 — 依頼一覧と端末の状態行が同じ規則で名乗るための唯一の owner。
pub use display_slug::display_slug_from_dir_name;

// コンパイル済み定義のファイル面 (`stage-graph.json` / `scope-grid.json` / `scopes/aidlc-*.md`)
// を読む実 Gateway。SQLite の `read_*` 表とは別の面なので `ReadModelDaos` の住人ではない
// (`state_file_dao_impl` と同型 — 配布データ置き場を握る)。
pub use scope_grid_dao_impl::ScopeGridDaoImpl;
pub use scope_metadata_dao_impl::ScopeMetadataDaoImpl;
pub use stage_graph_dao_impl::StageGraphDaoImpl;

// テスト用 in-memory 実装 (合成ルートとその周辺のテストが実 I/O 無しで組むための口)。
pub use memory::{
    InMemoryDefinitionDao, InMemoryDefinitionStageDao, InMemoryExecutionDao, InMemoryJumpDao,
    InMemoryJumpPhaseDao, InMemoryNextAnswerDao, InMemoryPhaseEntryDao, InMemoryRunStageDao,
    InMemoryScopeChangeDao, InMemoryScopeDao, InMemoryScopeKeywordDao, InMemorySteeringPartDao,
    InMemorySteeringPlanDao,
};

// 継続トークンの封緘・開封 (輸送形の境界)。
pub use continue_token_dto::{InvalidContinueToken, mint_continue_token, verify_continue_token};

mod report_result_dao_impl;
pub use report_result_dao_impl::ReportResultDaoImpl;

mod initialization_dao_impl;
pub use initialization_dao_impl::InitializationDaoImpl;

mod answer_result_dao_impl;
pub use answer_result_dao_impl::AnswerResultDaoImpl;
mod testing_contract_dao_impl;
pub use testing_contract_dao_impl::TestingContractDaoImpl;
mod plan_fingerprint_dao_impl;
pub use plan_fingerprint_dao_impl::PlanFingerprintDaoImpl;
mod code_generation_approval_dao_impl;
pub use code_generation_approval_dao_impl::CodeGenerationApprovalDaoImpl;

mod plan_approval_operation_dao_impl;
pub use plan_approval_operation_dao_impl::PlanApprovalOperationDaoImpl;

mod intent_record_dao_impl;
pub use intent_record_dao_impl::IntentRecordDaoImpl;

mod plan_answer_dao_impl;
pub use plan_answer_dao_impl::PlanAnswerDaoImpl;

mod plan_generation_dao_impl;
pub use plan_generation_dao_impl::PlanGenerationDaoImpl;

mod artifact_audit_dao_impl;
pub use artifact_audit_dao_impl::ArtifactAuditDaoImpl;
mod continuation_result_dao_impl;
pub use continuation_result_dao_impl::ContinuationResultDaoImpl;

mod session_audit_dao_impl;
pub use session_audit_dao_impl::SessionAuditDaoImpl;

mod pipeline_progress_dao_impl;
pub use pipeline_progress_dao_impl::PipelineProgressDaoImpl;

mod jump_result_dao_impl;
pub use jump_result_dao_impl::JumpResultDaoImpl;

// 自己診断 (`aidlc --doctor`) の観測 DAO と、その注入物 (U3 / C5・C7)。
pub use doctor_check_dao_impl::DoctorCheckDaoImpl;
pub use doctor_environment::DoctorEnvironment;
pub use doctor_observation_dao_impl::DoctorObservationDaoImpl;
pub use doctor_paths::DoctorPaths;
pub use doctor_report_dao_impl::DoctorReportDaoImpl;
pub use native_doctor_facts::NativeDoctorFacts;
pub use state_version_classifier::StateVersionClassifier;
