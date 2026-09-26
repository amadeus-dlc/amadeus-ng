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
//! Code Generation 開始可否 — PR2) と、Pipeline 面 (`read_pipeline_progress` — PR3) である。
//! 参照入力由来の面 (Pipeline 面を含む) はジャーナル上の位置ではなく行の `source_digest` で
//! 冪等を取るので、処理したシーケンス番号の表を持たない。残りの
//! 更新器の移行順は `aidlc/spaces/default/knowledge/rmu-dao-migration-plan-20260926.md` にある。
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

pub use code_generation_approval_dao::CodeGenerationApprovalDao;
pub use code_generation_approval_row::CodeGenerationApprovalRow;
pub use doctor_check_dao::DoctorCheckDao;
pub use doctor_check_row::DoctorCheckRow;
pub use doctor_report_dao::DoctorReportDao;
pub use doctor_report_row::DoctorReportRow;
pub use pipeline_progress_dao::PipelineProgressDao;
pub use pipeline_progress_row::PipelineProgressRow;
pub use plan_fingerprint_dao::PlanFingerprintDao;
pub use plan_fingerprint_row::PlanFingerprintRow;
pub use source_stamp::SourceStamp;
pub use steering_part_dao::SteeringPartDao;
pub use steering_part_row::SteeringPartRow;
pub use steering_plan_dao::SteeringPlanDao;
pub use steering_plan_row::SteeringPlanRow;
pub use testing_contract_dao::TestingContractDao;
pub use testing_contract_row::TestingContractRow;
pub use workspace_doctor_journal_entry::WorkspaceDoctorJournalEntry;
pub use workspace_doctor_journal_reader::WorkspaceDoctorJournalReader;
pub use workspace_doctor_projection_checkpoint_dao::WorkspaceDoctorProjectionCheckpointDao;
