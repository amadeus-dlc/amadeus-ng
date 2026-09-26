//! orchestration コンテキストの**読取語彙と取得ループ** — 差分読取・投影チェックポイント
//! （C3 / C6）と、その上に立つ RMU の取得ループ。
//!
//! ポート（`JournalReader`）も SQLite 実装（`JournalReaderImpl`）も**このクレートが所有する**。
//! 呼ぶのは RMU だけであり、ジャーナルを読むことが RMU の仕事そのものだからである
//! （2026-08-28 / 2026-08-29 裁定 — ADR-009）。中立クレートへ切り出す必要は無い。
//!
//! # 取得ループは 2 系統のリードモデルを 1 回で描く
//!
//! Markdown 面（系統 (1) — `aidlc-state.md` と監査シャード）は [`crate::workspace`] の投影核が
//! 描き、構造化面（系統 (2) — SQLite の `read_*` 表）は [`crate::read_tables`] の投影核が描く。
//! 構造化面の行は表ごとの DAO が書き、**20 表の差し替えと処理したシーケンス番号の前進は
//! 1 トランザクション**に閉じる（裁定 §3 — [`StructuredReadModelUpdater`] と、同じ手順を
//! 公開の確定の中で使う `JournalReader::publish`）。ジャーナルの読み手は行を受け取らない
//! （Issue #153 の PR4 で `advance_checkpoint` を取り除いた）。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_read_model_updater::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod code_generation_approval_read_model_updater;
mod corrupt_cause;
mod definition_entry;
mod dto;
mod global_seq_nr;
mod journal_batch;
mod journal_entry;
mod journal_read_error;
mod journal_reader;
mod journal_reader_impl;
mod orchestration_read_model_updater;
mod plan_fingerprint_read_model_updater;
mod projection_name;
mod projection_name_error;
mod projection_targets;
mod publication_batch;
mod publication_file;
mod publication_store;
mod read_model_update_error;
mod read_model_updater;
mod runtime_graph_read_model_updater;
mod runtime_graph_targets;
mod steering_source;
mod store_failure;
mod structured_read_model_updater;
mod testing_read_model_updater;

// ポート (trait) と実 I/O 実装
pub use journal_reader::JournalReader;
pub use journal_reader_impl::JournalReaderImpl;

// リードモデル更新の共通契約 (RMU のすべての更新入口が実装する)
pub use read_model_updater::ReadModelUpdater;

// 取得ループ (RMU コンポーネント本体 — 二層構造の上側) と、面ごとの更新器
pub use code_generation_approval_read_model_updater::CodeGenerationApprovalReadModelUpdater;
pub use orchestration_read_model_updater::OrchestrationReadModelUpdater;
pub use plan_fingerprint_read_model_updater::PlanFingerprintReadModelUpdater;
pub use projection_targets::ProjectionTargets;
pub use publication_batch::PublicationBatch;
pub use publication_file::PublicationFile;
pub use runtime_graph_read_model_updater::RuntimeGraphReadModelUpdater;
pub use runtime_graph_targets::RuntimeGraphTargets;
pub use steering_source::SteeringSource;
pub use structured_read_model_updater::StructuredReadModelUpdater;
pub use testing_read_model_updater::TestingReadModelUpdater;

// Domain Primitive (永続化の通番と投影の名前)
pub use global_seq_nr::GlobalSeqNr;
pub use projection_name::ProjectionName;

// ポートが返す読取レコード (本家の封筒型はポートから出さない — ADR-009 2026-08-28 追記)
pub use definition_entry::DefinitionEntry;
pub use journal_batch::JournalBatch;
pub use journal_entry::JournalEntry;

// エラー
// 読む側の永続化 DTO (側ごと専用化 — coding-rules/cqrs-boundaries.md)。
pub use dto::{
    AutonomyModeSetDto, DtoDecodeError, GateApprovedDto, GateOpenedDto, GateRejectedDto,
    IntentEventDto, IntentExecutionEventDto, JumpedDto, ParkedDto, PracticesAffirmedDto,
    RecomposedDto, SingleStageRunCommittedDto, SkeletonStanceRecordedDto, StageRevisedDto,
    StageSkippedDto, StartedDto, TaskSynchronizedDto, WorkflowDefinitionEventDto,
};

pub use corrupt_cause::CorruptCause;
pub use journal_read_error::JournalReadError;
pub use projection_name_error::ProjectionNameError;
pub use read_model_update_error::ReadModelUpdateError;

mod intent_registry;
mod plan_source;
pub use plan_source::PlanSource;

mod plan_approval_journal_reader;
pub use plan_approval_journal_reader::{PlanApprovalJournalReader, plan_approval_receipts};

mod plan_approval_journal_reader_impl;
pub use plan_approval_journal_reader_impl::PlanApprovalJournalReaderImpl;

mod plan_approval_journal_entry;
pub use plan_approval_journal_entry::PlanApprovalJournalEntry;

mod plan_approval_read_model_updater;
pub use plan_approval_read_model_updater::PlanApprovalReadModelUpdater;

mod plan_approval_files;

mod hook_health_read_model_updater;
pub use hook_health_read_model_updater::HookHealthReadModelUpdater;

mod artifact_journal_entry;
pub use artifact_journal_entry::ArtifactJournalEntry;
mod workflow_continuation_read_model_updater;
pub use workflow_continuation_read_model_updater::WorkflowContinuationReadModelUpdater;

pub use dto::PipelineLinkCompletedDto;

mod session_journal_entry;
pub use session_journal_entry::SessionJournalEntry;
mod session_event_dto;

pub use dto::SingleStageRunStartedDto;

// 更新器の構造 — ジャーナルを読む → 投影 → 表の DAO で更新する
// (`coding-rules/read-model-updater-structure.md`)。ポート (trait と、それが運ぶ値) は
// `port/`、実装は `…Impl` としてこの階層に置く。移行済みは自己診断 (Issue #153 の PR1) と、
// 参照入力由来の単独面 — steering・テスト契約・計画指紋・Code Generation 開始可否 (PR2) と、
// Pipeline 面 (PR3) と、構造化面 (ジャーナル由来の read_* 20 表・処理したシーケンス番号・共有面の
// 記録 — PR4)。
mod port;
pub use port::{
    AnswerResultRow, ArtifactAuditRow, CodeGenerationApprovalDao, CodeGenerationApprovalRow,
    DefinitionRow, DefinitionScopeKeywordRow, DefinitionScopePhaseEntryRow, DefinitionScopeRow,
    DefinitionScopeStageRow, DefinitionStageRow, DoctorCheckDao, DoctorCheckRow, DoctorReportDao,
    DoctorReportRow, ExecutionRow, ExecutionStageRow, IntentRow, IntentStageRow, JumpResultRow,
    NextAnswerRow, NextJumpPhaseRow, NextJumpRow, PipelineProgressDao, PipelineProgressRow,
    PlanFingerprintDao, PlanFingerprintRow, ReportResultRow, RunStageRow, ScopeChangeRow,
    SessionAuditRow, SourceStamp, SteeringPartDao, SteeringPartRow, SteeringPlanDao,
    SteeringPlanRow, TestingContractDao, TestingContractRow, WorkspaceDoctorJournalEntry,
    WorkspaceDoctorJournalReader, WorkspaceDoctorProjectionCheckpointDao,
};

mod updater_connection;

mod code_generation_approval_dao_impl;
mod plan_fingerprint_dao_impl;
mod steering_part_dao_impl;
mod steering_plan_dao_impl;
mod testing_contract_dao_impl;
pub use code_generation_approval_dao_impl::CodeGenerationApprovalDaoImpl;
pub use plan_fingerprint_dao_impl::PlanFingerprintDaoImpl;
pub use steering_part_dao_impl::SteeringPartDaoImpl;
pub use steering_plan_dao_impl::SteeringPlanDaoImpl;
pub use testing_contract_dao_impl::TestingContractDaoImpl;

mod steering_read_model_updater;
pub use steering_read_model_updater::SteeringReadModelUpdater;

mod pipeline_progress_dao_impl;
mod pipeline_progress_read_model_updater;
pub use pipeline_progress_dao_impl::PipelineProgressDaoImpl;
pub use pipeline_progress_read_model_updater::PipelineProgressReadModelUpdater;

mod doctor_check_dao_impl;
mod doctor_report_dao_impl;
mod workspace_doctor_journal_reader_impl;
mod workspace_doctor_projection_checkpoint_dao_impl;
pub use doctor_check_dao_impl::DoctorCheckDaoImpl;
pub use doctor_report_dao_impl::DoctorReportDaoImpl;
pub use workspace_doctor_journal_reader_impl::WorkspaceDoctorJournalReaderImpl;
pub use workspace_doctor_projection_checkpoint_dao_impl::WorkspaceDoctorProjectionCheckpointDaoImpl;

mod workspace_doctor_read_model_updater;
pub use workspace_doctor_read_model_updater::WorkspaceDoctorReadModelUpdater;

// 構造化面 (Issue #153 の PR4) — 20 表の DAO と、処理したシーケンス番号・共有面の記録・読み面の
// 版の DAO、構造化面の更新器が使うジャーナルの読み手。書く手順 (表をまたぐ検査を含む) は
// `structured_surface`、読み面の表の用意 (版を含む) は `read_model_schema` の 1 か所が持つ。
pub use port::{
    AnswerResultDao, ArtifactAuditDao, DefinitionDao, DefinitionScopeDao,
    DefinitionScopeKeywordDao, DefinitionScopePhaseEntryDao, DefinitionScopeStageDao,
    DefinitionStageDao, ExecutionDao, ExecutionStageDao, IntentDao, IntentStageDao, JournalAnchor,
    JumpResultDao, NextAnswerDao, NextJumpDao, NextJumpPhaseDao, ProjectionCheckpointDao,
    ProjectionCheckpointRow, ReadModelHeadDao, ReadModelHeadRow, ReadSchemaVersionDao,
    ReportResultDao, RunStageDao, ScopeChangeDao, SessionAuditDao, StructuredJournalReader,
    TableContent,
};

mod column_value;
mod read_model_schema;
mod structured_surface;
mod structured_surface_content;

mod answer_result_dao_impl;
mod artifact_audit_dao_impl;
mod definition_dao_impl;
mod definition_scope_dao_impl;
mod definition_scope_keyword_dao_impl;
mod definition_scope_phase_entry_dao_impl;
mod definition_scope_stage_dao_impl;
mod definition_stage_dao_impl;
mod execution_dao_impl;
mod execution_stage_dao_impl;
mod intent_dao_impl;
mod intent_stage_dao_impl;
mod jump_result_dao_impl;
mod next_answer_dao_impl;
mod next_jump_dao_impl;
mod next_jump_phase_dao_impl;
mod projection_checkpoint_dao_impl;
mod read_model_head_dao_impl;
mod read_schema_version_dao_impl;
mod report_result_dao_impl;
mod run_stage_dao_impl;
mod scope_change_dao_impl;
mod session_audit_dao_impl;
mod structured_journal_reader_impl;
pub use answer_result_dao_impl::AnswerResultDaoImpl;
pub use artifact_audit_dao_impl::ArtifactAuditDaoImpl;
pub use definition_dao_impl::DefinitionDaoImpl;
pub use definition_scope_dao_impl::DefinitionScopeDaoImpl;
pub use definition_scope_keyword_dao_impl::DefinitionScopeKeywordDaoImpl;
pub use definition_scope_phase_entry_dao_impl::DefinitionScopePhaseEntryDaoImpl;
pub use definition_scope_stage_dao_impl::DefinitionScopeStageDaoImpl;
pub use definition_stage_dao_impl::DefinitionStageDaoImpl;
pub use execution_dao_impl::ExecutionDaoImpl;
pub use execution_stage_dao_impl::ExecutionStageDaoImpl;
pub use intent_dao_impl::IntentDaoImpl;
pub use intent_stage_dao_impl::IntentStageDaoImpl;
pub use jump_result_dao_impl::JumpResultDaoImpl;
pub use next_answer_dao_impl::NextAnswerDaoImpl;
pub use next_jump_dao_impl::NextJumpDaoImpl;
pub use next_jump_phase_dao_impl::NextJumpPhaseDaoImpl;
pub use projection_checkpoint_dao_impl::ProjectionCheckpointDaoImpl;
pub use read_model_head_dao_impl::ReadModelHeadDaoImpl;
pub use read_schema_version_dao_impl::ReadSchemaVersionDaoImpl;
pub use report_result_dao_impl::ReportResultDaoImpl;
pub use run_stage_dao_impl::RunStageDaoImpl;
pub use scope_change_dao_impl::ScopeChangeDaoImpl;
pub use session_audit_dao_impl::SessionAuditDaoImpl;
pub use structured_journal_reader_impl::StructuredJournalReaderImpl;

#[cfg(test)]
mod structured_surface_tables_tests;
