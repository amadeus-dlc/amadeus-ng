//! RMU のアウトプットポート — 更新器が依存する**契約 (trait)** と、その契約が運ぶ値の置き場。
//! 配置はクエリ側の `port/` と同型である (`coding-rules/read-model-updater-structure.md`)。
//!
//! # ポートは構造上の 1 要素の代理である
//!
//! 更新器の仕事は「ジャーナルを読む → 投影 (純粋な変換) → DAO でリードモデルを更新する」
//! だけである (オーナー裁定 2026-09-26)。ポートはその構造の要素ごとに 1 本ずつ立てる。
//!
//! - **ジャーナルの読み手** (`…JournalReader`) — ある位置より後／までの事実を読むだけ。
//!   リードモデル側の型 (行・表) に依存しない。
//! - **表の DAO** (`<表名>Dao`) — 単一テーブルの I/O だけ。表をまたぐ検査・導出は持たない
//!   (それは更新器かドメイン／値の型の仕事)。チェックポイントの表も表の 1 つであり、
//!   専用の DAO を持つ。
//!
//! 「リードモデルへ書く」大きなポート (`ReadModelWriter` のようなもの) は作らない。
//!
//! # トランザクションは更新器が持つ
//!
//! 複数の表と処理したシーケンス番号を 1 つの DB トランザクションで確定するため、表の DAO は
//! 接続を持たない。更新器が `BEGIN IMMEDIATE` で開いたトランザクションを `&mut` で受け取って
//! 書き、読取は `&Connection` (トランザクションはこれに参照外しされる) で行う。確定と
//! 取り消しは更新器が決める。
//!
//! # 移行中である
//!
//! 2026-09-26 時点でこの形へ移したのは、自己診断 (`WorkspaceDoctorReadModelUpdater` —
//! Issue #153 の PR1) と、参照入力由来の単独面 (steering・テスト契約・計画指紋・
//! Code Generation 開始可否 — PR2) と、Pipeline 面 (`read_pipeline_progress` — PR3) と、
//! 構造化面 (ジャーナル由来の `read_*` 20 表・処理したシーケンス番号
//! `amadeus_projection_checkpoint`・共有面の記録 `amadeus_read_model_head`・読み面の版
//! `PRAGMA user_version` — PR4) である。
//! 参照入力由来の面 (Pipeline 面を含む) はジャーナル上の位置ではなく行の `source_digest` で
//! 冪等を取るので、処理したシーケンス番号の表を持たない。構造化面は番号の表を持ち、20 表と
//! 同じ IMMEDIATE トランザクションで確定する。
//!
//! 構造化面の更新器が使うジャーナルの読み手 (`StructuredJournalReader`) は、自己診断の
//! `WorkspaceDoctorJournalReader` と同じく移行中の面ごとの読み手である (旧 `JournalReader` は
//! まだ公開の書込を抱えている)。最後に `JournalReader` 1 本へまとめる。
//!
//! 管理表の DAO の名前から名前空間の接頭辞 `amadeus_` を除く (`ProjectionCheckpointDao` /
//! `ReadModelHeadDao`) のは、`read_` を除くのと同じ理由である — 本家の表と衝突しないための
//! 接頭辞であって、表の意味ではない。
//!
//! 残りの更新器の移行順は `aidlc/spaces/default/knowledge/rmu-dao-migration-plan-20260926.md` にある。
//!
//! 型ファイルの mod も本モジュール自身も private。公開 API は親 (`orchestration`) の
//! `pub use` ファサードが唯一の宣言 (`coding-rules/module-visibility.md`)。

// ジャーナルの読み手と、それが返す読取レコード
mod workspace_doctor_journal_entry;
mod workspace_doctor_journal_reader;

// 表の DAO と、それが書く行
mod doctor_check_dao;
mod doctor_check_row;
mod doctor_report_dao;
mod doctor_report_row;
mod workspace_doctor_projection_checkpoint_dao;

// 参照入力由来の面の表の DAO と、それが書く行・読む出所 (行を組む投影は `read_tables`)
mod code_generation_approval_dao;
mod code_generation_approval_row;
mod plan_fingerprint_dao;
mod plan_fingerprint_row;
mod source_stamp;
mod steering_part_dao;
mod steering_part_row;
mod steering_plan_dao;
mod steering_plan_row;
mod testing_contract_dao;
mod testing_contract_row;

// Pipeline 面の表の DAO と、それが書く行 (行を組む投影は `read_tables::PipelineTables`)
mod pipeline_progress_dao;
mod pipeline_progress_row;

// 構造化面 (ジャーナル由来の read_* 20 表) の DAO が運ぶ行 (行を組む投影は
// read_tables::ReadTables::project)
mod answer_result_row;
mod artifact_audit_row;
mod definition_row;
mod definition_scope_keyword_row;
mod definition_scope_phase_entry_row;
mod definition_scope_row;
mod definition_scope_stage_row;
mod definition_stage_row;
mod execution_row;
mod execution_stage_row;
mod intent_row;
mod intent_stage_row;
mod jump_result_row;
mod next_answer_row;
mod next_jump_phase_row;
mod next_jump_row;
mod report_result_row;
mod run_stage_row;
mod scope_change_row;
mod session_audit_row;

// 構造化面 (ジャーナル由来の read_* 20 表) の表の DAO と、処理したシーケンス番号・共有面の記録・
// 読み面の版の DAO、それらが運ぶ値、構造化面の更新器が使うジャーナルの読み手 (Issue #153 の PR4)
mod answer_result_dao;
mod artifact_audit_dao;
mod definition_dao;
mod definition_scope_dao;
mod definition_scope_keyword_dao;
mod definition_scope_phase_entry_dao;
mod definition_scope_stage_dao;
mod definition_stage_dao;
mod execution_dao;
mod execution_stage_dao;
mod intent_dao;
mod intent_stage_dao;
mod journal_anchor;
mod jump_result_dao;
mod next_answer_dao;
mod next_jump_dao;
mod next_jump_phase_dao;
mod projection_checkpoint_dao;
mod projection_checkpoint_row;
mod read_model_head_dao;
mod read_model_head_row;
mod read_schema_version_dao;
mod report_result_dao;
mod run_stage_dao;
mod scope_change_dao;
mod session_audit_dao;
mod structured_journal_reader;
mod table_content;

pub use answer_result_dao::AnswerResultDao;
pub use answer_result_row::AnswerResultRow;
pub use artifact_audit_dao::ArtifactAuditDao;
pub use artifact_audit_row::ArtifactAuditRow;
pub use code_generation_approval_dao::CodeGenerationApprovalDao;
pub use code_generation_approval_row::CodeGenerationApprovalRow;
pub use definition_dao::DefinitionDao;
pub use definition_row::DefinitionRow;
pub use definition_scope_dao::DefinitionScopeDao;
pub use definition_scope_keyword_dao::DefinitionScopeKeywordDao;
pub use definition_scope_keyword_row::DefinitionScopeKeywordRow;
pub use definition_scope_phase_entry_dao::DefinitionScopePhaseEntryDao;
pub use definition_scope_phase_entry_row::DefinitionScopePhaseEntryRow;
pub use definition_scope_row::DefinitionScopeRow;
pub use definition_scope_stage_dao::DefinitionScopeStageDao;
pub use definition_scope_stage_row::DefinitionScopeStageRow;
pub use definition_stage_dao::DefinitionStageDao;
pub use definition_stage_row::DefinitionStageRow;
pub use doctor_check_dao::DoctorCheckDao;
pub use doctor_check_row::DoctorCheckRow;
pub use doctor_report_dao::DoctorReportDao;
pub use doctor_report_row::DoctorReportRow;
pub use execution_dao::ExecutionDao;
pub use execution_row::ExecutionRow;
pub use execution_stage_dao::ExecutionStageDao;
pub use execution_stage_row::ExecutionStageRow;
pub use intent_dao::IntentDao;
pub use intent_row::IntentRow;
pub use intent_stage_dao::IntentStageDao;
pub use intent_stage_row::IntentStageRow;
pub use journal_anchor::JournalAnchor;
pub use jump_result_dao::JumpResultDao;
pub use jump_result_row::JumpResultRow;
pub use next_answer_dao::NextAnswerDao;
pub use next_answer_row::NextAnswerRow;
pub use next_jump_dao::NextJumpDao;
pub use next_jump_phase_dao::NextJumpPhaseDao;
pub use next_jump_phase_row::NextJumpPhaseRow;
pub use next_jump_row::NextJumpRow;
pub use pipeline_progress_dao::PipelineProgressDao;
pub use pipeline_progress_row::PipelineProgressRow;
pub use plan_fingerprint_dao::PlanFingerprintDao;
pub use plan_fingerprint_row::PlanFingerprintRow;
pub use projection_checkpoint_dao::ProjectionCheckpointDao;
pub use projection_checkpoint_row::ProjectionCheckpointRow;
pub use read_model_head_dao::ReadModelHeadDao;
pub use read_model_head_row::ReadModelHeadRow;
pub use read_schema_version_dao::ReadSchemaVersionDao;
pub use report_result_dao::ReportResultDao;
pub use report_result_row::ReportResultRow;
pub use run_stage_dao::RunStageDao;
pub use run_stage_row::RunStageRow;
pub use scope_change_dao::ScopeChangeDao;
pub use scope_change_row::ScopeChangeRow;
pub use session_audit_dao::SessionAuditDao;
pub use session_audit_row::SessionAuditRow;
pub use source_stamp::SourceStamp;
pub use steering_part_dao::SteeringPartDao;
pub use steering_part_row::SteeringPartRow;
pub use steering_plan_dao::SteeringPlanDao;
pub use steering_plan_row::SteeringPlanRow;
pub use structured_journal_reader::StructuredJournalReader;
pub use table_content::TableContent;
pub use testing_contract_dao::TestingContractDao;
pub use testing_contract_row::TestingContractRow;
pub use workspace_doctor_journal_entry::WorkspaceDoctorJournalEntry;
pub use workspace_doctor_journal_reader::WorkspaceDoctorJournalReader;
pub use workspace_doctor_projection_checkpoint_dao::WorkspaceDoctorProjectionCheckpointDao;
