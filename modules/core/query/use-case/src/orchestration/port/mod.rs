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
//! 例外は `StateFileDao` 1 本である — 引く先が `read_*` 表ではなく **upstream 互換の
//! 人間可読リードモデル** (`aidlc-state.md`) なので、返すのは列の写しではなく生テキストで
//! ある。読取動詞しか持たない点は同じである。
//!
//! # 1 表 1 ポート 1 View
//!
//! 13 のポートはそれぞれ `read_*` 表を**ちょうど 1 つ**引く (オーナー裁定 2026-09-03 —
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
mod codekb_scope_dao;
mod codekb_source_fingerprint_dao;
mod definition_dao;
mod definition_stage_dao;
mod doctor_check_dao;
mod doctor_observation_dao;
mod doctor_report_dao;
mod doctor_view;
mod document_input_bytes;
mod document_input_dao;
mod document_input_read_error;
mod document_input_view;
mod execution_dao;
mod intent_listing_dao;
mod intent_listing_row_view;
mod intent_repos_dao;
mod jump_dao;
mod jump_phase_dao;
mod next_answer_dao;
mod phase_entry_dao;
mod project_description_dao;
mod read_model_read_error;
mod run_stage_dao;
mod scope_change_dao;
mod scope_dao;
mod scope_grid_dao;
mod scope_keyword_dao;
mod scope_metadata_dao;
mod stage_graph_dao;
mod state_file_dao;
mod steering_part_dao;
mod steering_plan_dao;

// 契約が返す DTO (同居 — オーナー裁定 2026-08-31)
mod read_view;

pub use codekb_scope_dao::CodekbScopeDao;
pub use codekb_source_fingerprint_dao::CodekbSourceFingerprintDao;
pub use definition_dao::DefinitionDao;
pub use definition_stage_dao::DefinitionStageDao;
pub use doctor_check_dao::DoctorCheckDao;
pub use doctor_observation_dao::DoctorObservationDao;
pub use doctor_report_dao::DoctorReportDao;
pub use doctor_view::{
    ConsumeView, DefinitionAssetsView, DoctorCheck, DoctorObservationView, DoctorReport,
    DoctorSummaryView, ExecutionCursorView, GraphStageView, HeartbeatEntryView, HeartbeatView,
    HookBindingDeclaration, HookBindingTarget, HookBindingView, HookWiringView,
    NativeEntryPointsView, ObservationFailure, ProjectionObservationView, RecordLocationView,
    RecordObservationView, ScopeGridEntryView, StageArtifactsView, StageFileView,
    StateFileObservationView, StateVersionKindView, StateVersionView, StoreObservationView,
    StoreSchemaView, TimestampView, WiredHookView, WorkspaceShellView,
};
pub use document_input_bytes::DocumentInputBytes;
pub use document_input_dao::DocumentInputDao;
pub use document_input_read_error::DocumentInputReadError;
pub use document_input_view::DocumentInputView;
pub use execution_dao::ExecutionDao;
pub use intent_listing_dao::IntentListingDao;
pub use intent_listing_row_view::IntentListingRowView;
pub use intent_repos_dao::IntentReposDao;
pub use jump_dao::JumpDao;
pub use jump_phase_dao::JumpPhaseDao;
pub use next_answer_dao::NextAnswerDao;
pub use phase_entry_dao::PhaseEntryDao;
pub use project_description_dao::ProjectDescriptionDao;
pub use run_stage_dao::RunStageDao;
pub use scope_change_dao::ScopeChangeDao;
pub use scope_dao::ScopeDao;
pub use scope_grid_dao::ScopeGridDao;
pub use scope_keyword_dao::ScopeKeywordDao;
pub use scope_metadata_dao::ScopeMetadataDao;
pub use stage_graph_dao::StageGraphDao;
pub use state_file_dao::StateFileDao;
pub use steering_part_dao::SteeringPartDao;
pub use steering_plan_dao::SteeringPlanDao;

pub use read_model_read_error::ReadModelReadError;

pub use read_view::{
    CodekbScopeDiffView, DefinitionStageView, DefinitionSummaryView, ExecutionView, JumpPhaseView,
    JumpView, NextAnswerView, PhaseEntryView, ProjectDescriptionView, ReScopeParseView,
    ReScopeView, RunStageView, ScopeActionsView, ScopeCatalogRowView, ScopeChangeView,
    ScopeMetadataView, ScopeView, StageGraphEntryView, SteeringPartView, SteeringPlanView,
};

mod report_result_dao;
mod report_result_view;
pub use report_result_dao::ReportResultDao;
pub use report_result_view::ReportResultView;

mod initialization_dao;
mod initialization_view;
pub use initialization_dao::InitializationDao;
pub use initialization_view::InitializationView;

mod answer_result_view;
pub use answer_result_view::AnswerResultView;
mod answer_result_dao;
pub use answer_result_dao::AnswerResultDao;
mod testing_contract_view;
pub use testing_contract_view::TestingContractView;
mod testing_contract_dao;
pub use testing_contract_dao::TestingContractDao;
mod plan_fingerprint_view;
pub use plan_fingerprint_view::PlanFingerprintView;
mod plan_fingerprint_dao;
pub use plan_fingerprint_dao::PlanFingerprintDao;
mod code_generation_approval_view;
pub use code_generation_approval_view::CodeGenerationApprovalView;
mod code_generation_approval_dao;
pub use code_generation_approval_dao::CodeGenerationApprovalDao;

mod plan_approval_operation_view;
pub use plan_approval_operation_view::PlanApprovalOperationView;
mod plan_approval_operation_dao;
pub use plan_approval_operation_dao::PlanApprovalOperationDao;

mod intent_record_view;
pub use intent_record_view::IntentRecordView;
mod intent_record_dao;
pub use intent_record_dao::IntentRecordDao;

mod plan_answer_dao;
mod plan_answer_view;
pub use plan_answer_dao::PlanAnswerDao;
pub use plan_answer_view::PlanAnswerView;

mod plan_generation_dao;
mod plan_generation_view;
pub use plan_generation_dao::PlanGenerationDao;
pub use plan_generation_view::PlanGenerationView;

mod hook_health_dao;
mod hook_health_view;
pub use hook_health_dao::HookHealthDao;
pub use hook_health_view::HookHealthView;

mod artifact_audit_dao;
mod artifact_audit_view;
pub use artifact_audit_dao::ArtifactAuditDao;
pub use artifact_audit_view::ArtifactAuditView;
mod continuation_result_dao;
mod continuation_result_view;
pub use continuation_result_dao::ContinuationResultDao;
pub use continuation_result_view::ContinuationResultView;

mod session_audit_view;
pub use session_audit_view::SessionAuditView;
mod session_audit_dao;
pub use session_audit_dao::SessionAuditDao;

mod pipeline_progress_view;
pub use pipeline_progress_view::PipelineProgressView;
mod pipeline_progress_dao;
pub use pipeline_progress_dao::PipelineProgressDao;

mod jump_result_dao;
mod jump_result_view;
pub use jump_result_dao::JumpResultDao;
pub use jump_result_view::JumpResultView;
